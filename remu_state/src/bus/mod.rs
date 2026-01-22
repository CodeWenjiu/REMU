remu_macro::mod_flat!(error, command, option, memory, device);

use std::ops::Range;

pub use memory::MemRegionSpec;
use remu_types::{AllUsize, DynDiagError};

// Bring object traits into scope for `File::segments()` and `Segment::{address,size,data}`.
use object::{Object as _, ObjectSegment as _};

// Use the public re-export to avoid shadowing the glob re-exported `Memory`

fn try_load_elf_into_memory(
    memory: &mut [Memory],
    elf: &Option<std::path::PathBuf>,
    tracer: &remu_types::TracerDyn,
) {
    // Optional ELF loading: best-effort only.
    //
    // Behavior:
    // - If --elf is not provided: do nothing.
    // - If provided but missing/unreadable/invalid: print a message via tracer and continue.
    // - If valid ELF but no mapped region can contain it at its start address: print message and continue.
    // - If it fits: copy bytes into the matching region.
    let Some(path) = elf.as_ref() else {
        return;
    };

    // Even though clap validates this today, keep behavior robust for programmatic uses.
    if !path.exists() {
        tracer
            .borrow()
            .print(&format!("ELF path does not exist: {}", path.display()));
        return;
    }
    if !path.is_file() {
        tracer
            .borrow()
            .print(&format!("ELF path is not a file: {}", path.display()));
        return;
    }

    let buf = match std::fs::read(path) {
        Ok(b) => b,
        Err(err) => {
            tracer.borrow().print(&format!(
                "Failed to read ELF file '{}': {err}",
                path.display()
            ));
            return;
        }
    };

    let obj = match object::File::parse(buf.as_slice()) {
        Ok(o) => o,
        Err(err) => {
            tracer
                .borrow()
                .print(&format!("Failed to parse ELF '{}': {err}", path.display()));
            return;
        }
    };

    // Compute the overall loaded image range based on segment VAs:
    // start = min(seg.address)
    // end   = max(seg.address + seg.size)
    //
    // NOTE: We don't try to interpret "loadable" flags here; we just use the
    // segments() iterator as exposed by the object crate.
    let mut any_seg = false;
    let mut start: u64 = u64::MAX;
    let mut end: u64 = 0;

    for seg in obj.segments() {
        let size = seg.size();
        if size == 0 {
            continue;
        }
        any_seg = true;
        let addr = seg.address();
        start = start.min(addr);
        end = end.max(addr.saturating_add(size));
    }

    if !any_seg || start == u64::MAX || end <= start {
        tracer
            .borrow()
            .print(&format!("ELF has no loadable segments: {}", path.display()));
        return;
    }

    let start_usize = start as usize;
    let end_usize = end as usize;
    let total_len = end_usize.saturating_sub(start_usize);

    // Find a mapped memory region that can contain [start, end).
    let mut region_idx: Option<usize> = None;
    for (i, m) in memory.iter().enumerate() {
        if start_usize >= m.range.start && end_usize <= m.range.end {
            region_idx = Some(i);
            break;
        }
    }

    let Some(i) = region_idx else {
        tracer.borrow().print(&format!(
            "No mapped memory region can contain ELF image [{:#x}:{:#x}) ({} bytes) from {}",
            start,
            end,
            total_len,
            path.display()
        ));
        return;
    };

    // Copy each segment's file-backed bytes into memory at segment VA.
    for seg in obj.segments() {
        // seg.data() returns the bytes present in the file for the segment.
        // (This corresponds to filesz; BSS will typically not be included.)
        let seg_bytes = match seg.data() {
            Ok(b) => b,
            Err(err) => {
                tracer.borrow().print(&format!(
                    "Failed to read ELF segment bytes from {}: {err}",
                    path.display()
                ));
                continue;
            }
        };

        if seg_bytes.is_empty() {
            continue;
        }

        let addr = seg.address() as usize;

        if addr >= memory[i].range.start && addr + seg_bytes.len() <= memory[i].range.end {
            memory[i].write_bytes(addr, seg_bytes);
        } else {
            tracer.borrow().print(&format!(
                "ELF segment does not fit mapped region '{}': addr={:#x}, len={} (region [{:#x}:{:#x}))",
                memory[i].name,
                addr,
                seg_bytes.len(),
                memory[i].range.start,
                memory[i].range.end
            ));
        }
    }

    tracer.borrow().print(&format!(
        "Loaded ELF into memory region '{}' at [{:#x}:{:#x}) from {}",
        memory[i].name,
        start,
        end,
        path.display()
    ));
}

pub struct Bus {
    memory: Box<[Memory]>,
    /// Last-hit cache for region lookup.
    ///
    /// This is an extremely effective fast-path when workloads exhibit any locality
    /// (or when there is only a single region): we first check the previously-hit
    /// region and fall back to scanning only if it doesn't match.
    last_hit: Option<usize>,
    tracer: remu_types::TracerDyn,
}

impl Bus {
    pub(crate) fn new(opt: BusOption, tracer: remu_types::TracerDyn) -> Self {
        let memory: Vec<Memory> = opt
            .mem
            .into_iter()
            .map(|region| {
                Memory::new(region)
                    .expect("invalid memory region spec (should be validated before Bus::new)")
            })
            .collect();

        let mut memory = memory.into_boxed_slice();

        // Keep Bus::new small; perform best-effort ELF loading in a helper.
        try_load_elf_into_memory(&mut memory, &opt.elf, &tracer);

        Self {
            memory,
            last_hit: None,
            tracer,
        }
    }

    /// Return a snapshot of the current memory map as (name, address range) pairs.
    ///
    /// This is intended for frontends (via `State` -> `Tracer`) to render a memory map table.
    /// The returned `Vec` is small (number of regions) and cheap to build.
    pub fn mem_map(&self) -> Vec<(String, Range<usize>)> {
        self.memory
            .iter()
            .map(|m| (m.name.clone(), m.range.clone()))
            .collect()
    }

    #[inline(always)]
    fn find_memory_mut(&mut self, range: Range<usize>) -> Result<&mut Memory, BusFault> {
        // Fast path: check last hit first.
        if let Some(i) = self.last_hit {
            // SAFETY: i must be a valid index into self.memory.
            if unsafe { self.memory.get_unchecked(i).contains(range.clone()) } {
                return Ok(&mut self.memory[i]);
            }
        }

        // Slow path: scan all regions.
        self.find_memory_mut_slow(range)
    }

    #[cold]
    #[inline(never)]
    fn find_memory_mut_slow(&mut self, range: Range<usize>) -> Result<&mut Memory, BusFault> {
        // First match wins. If you later allow overlapping regions, you must define priority.
        // For now, regions are expected to be non-overlapping.
        for (i, m) in self.memory.iter_mut().enumerate() {
            if m.contains(range.clone()) {
                self.last_hit = Some(i);
                return Ok(m);
            }
        }

        // If address isn't mapped, keep last_hit unchanged (it may still be useful).
        Err(BusFault::Unmapped { addr: range.start })
    }

    pub(crate) fn execute(&mut self, subcmd: &BusCmd) {
        match subcmd {
            BusCmd::Read { subcmd } => {
                let (addr, result) = match subcmd {
                    ReadCommand::U8(arg) => {
                        (arg.addr, self.read_8(arg.addr).map(|v| AllUsize::U8(v)))
                    }
                    ReadCommand::U16(arg) => {
                        (arg.addr, self.read_16(arg.addr).map(|v| AllUsize::U16(v)))
                    }
                    ReadCommand::U32(arg) => {
                        (arg.addr, self.read_32(arg.addr).map(|v| AllUsize::U32(v)))
                    }
                    ReadCommand::U64(arg) => {
                        (arg.addr, self.read_64(arg.addr).map(|v| AllUsize::U64(v)))
                    }
                    ReadCommand::U128(arg) => {
                        (arg.addr, self.read_128(arg.addr).map(|v| AllUsize::U128(v)))
                    }
                };
                self.tracer.borrow().mem_show(
                    addr,
                    result.map_err(|e| Box::new(e) as Box<dyn DynDiagError>),
                );
            }
            BusCmd::Print { addr, count } => {
                let mut buf = vec![0u8 as u8; *count];
                let result = self
                    .read_bytes(*addr, &mut buf)
                    .map_err(|e| Box::new(e) as Box<dyn DynDiagError>);
                self.tracer.borrow_mut().mem_print(*addr, &buf, result);
            }
            BusCmd::Write { subcmd } => {
                let result = match subcmd {
                    WriteCommand::U8 { addr, value } => self.write_8(*addr, *value),
                    WriteCommand::U16 { addr, value } => self.write_16(*addr, *value),
                    WriteCommand::U32 { addr, value } => self.write_32(*addr, *value),
                    WriteCommand::U64 { addr, value } => self.write_64(*addr, *value),
                    WriteCommand::U128 { addr, value } => self.write_128(*addr, *value),
                };
                if let Err(e) = result {
                    self.tracer
                        .borrow()
                        .deal_error(Box::new(e) as Box<dyn DynDiagError>)
                }
            }
            BusCmd::Set { address, value } => {
                let mut addr = *address;
                for chunk in value.iter() {
                    if chunk.is_empty() {
                        continue;
                    }
                    if let Err(e) = self.write_bytes(addr, chunk) {
                        self.tracer
                            .borrow()
                            .deal_error(Box::new(e) as Box<dyn DynDiagError>);
                        break;
                    }
                    addr = addr.saturating_add(chunk.len());
                }
            }
        }
    }
}

pub trait BusAccess {
    type Fault;

    fn read_8(&mut self, addr: usize) -> Result<u8, Self::Fault>;
    fn read_16(&mut self, addr: usize) -> Result<u16, Self::Fault>;
    fn read_32(&mut self, addr: usize) -> Result<u32, Self::Fault>;
    fn read_64(&mut self, addr: usize) -> Result<u64, Self::Fault>;
    fn read_128(&mut self, addr: usize) -> Result<u128, Self::Fault>;
    fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), Self::Fault>;

    fn write_8(&mut self, addr: usize, value: u8) -> Result<(), Self::Fault>;
    fn write_16(&mut self, addr: usize, value: u16) -> Result<(), Self::Fault>;
    fn write_32(&mut self, addr: usize, value: u32) -> Result<(), Self::Fault>;
    fn write_64(&mut self, addr: usize, value: u64) -> Result<(), Self::Fault>;
    fn write_128(&mut self, addr: usize, value: u128) -> Result<(), Self::Fault>;
    fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), Self::Fault>;
}

impl BusAccess for Bus {
    type Fault = BusFault;

    #[inline(always)]
    fn read_8(&mut self, addr: usize) -> Result<u8, Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 1)?;
        Ok(m.read_8(addr))
    }

    #[inline(always)]
    fn read_16(&mut self, addr: usize) -> Result<u16, Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 2)?;
        Ok(m.read_16(addr))
    }

    #[inline(always)]
    fn read_32(&mut self, addr: usize) -> Result<u32, Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 4)?;
        Ok(m.read_32(addr))
    }

    #[inline(always)]
    fn read_64(&mut self, addr: usize) -> Result<u64, Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        Ok(m.read_64(addr))
    }

    #[inline(always)]
    fn read_128(&mut self, addr: usize) -> Result<u128, Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.read_128(addr))
    }

    #[inline(always)]
    fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), Self::Fault> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.read_bytes(addr, buf))
    }

    #[inline(always)]
    fn write_8(&mut self, addr: usize, value: u8) -> Result<(), Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 1)?;
        Ok(m.write_8(addr, value))
    }

    #[inline(always)]
    fn write_16(&mut self, addr: usize, value: u16) -> Result<(), Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 2)?;
        Ok(m.write_16(addr, value))
    }

    #[inline(always)]
    fn write_32(&mut self, addr: usize, value: u32) -> Result<(), Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 4)?;
        Ok(m.write_32(addr, value))
    }

    #[inline(always)]
    fn write_64(&mut self, addr: usize, value: u64) -> Result<(), Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        Ok(m.write_64(addr, value))
    }

    #[inline(always)]
    fn write_128(&mut self, addr: usize, value: u128) -> Result<(), Self::Fault> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.write_128(addr, value))
    }

    #[inline(always)]
    fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), Self::Fault> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.write_bytes(addr, buf))
    }
}
