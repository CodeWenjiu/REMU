remu_macro::mod_flat!(error, command, option, memory, device);

use std::ops::Range;

pub use memory::MemRegionSpec;
use remu_types::{AllUsize, DynDiagError};

// Use the public re-export to avoid shadowing the glob re-exported `Memory`

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

        Self {
            memory: memory.into_boxed_slice(),
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
    fn find_memory_mut(&mut self, range: Range<usize>) -> Result<&mut Memory, Box<BusFault>> {
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
    fn find_memory_mut_slow(&mut self, range: Range<usize>) -> Result<&mut Memory, Box<BusFault>> {
        // First match wins. If you later allow overlapping regions, you must define priority.
        // For now, regions are expected to be non-overlapping.
        for (i, m) in self.memory.iter_mut().enumerate() {
            if m.contains(range.clone()) {
                self.last_hit = Some(i);
                return Ok(m);
            }
        }

        // If address isn't mapped, keep last_hit unchanged (it may still be useful).
        Err(Box::new(BusFault::Unmapped { addr: range.start }))
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
                self.tracer
                    .borrow()
                    .mem_show(addr, result.map_err(|e| e as Box<dyn DynDiagError>));
            }
            BusCmd::Print { addr, count } => {
                let mut buf = vec![0u8 as u8; *count];
                let result = self
                    .read_bytes(*addr, &mut buf)
                    .map_err(|e| e as Box<dyn DynDiagError>);
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
                    self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>)
                }
            }
            BusCmd::Set { address, value } => {
                let mut addr = *address;
                for chunk in value.iter() {
                    if chunk.is_empty() {
                        continue;
                    }
                    if let Err(e) = self.write_bytes(addr, chunk) {
                        self.tracer.borrow().deal_error(e as Box<dyn DynDiagError>);
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

    fn read_8(&mut self, addr: usize) -> Result<u8, Box<Self::Fault>>;
    fn read_16(&mut self, addr: usize) -> Result<u16, Box<Self::Fault>>;
    fn read_32(&mut self, addr: usize) -> Result<u32, Box<Self::Fault>>;
    fn read_64(&mut self, addr: usize) -> Result<u64, Box<Self::Fault>>;
    fn read_128(&mut self, addr: usize) -> Result<u128, Box<Self::Fault>>;
    fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), Box<Self::Fault>>;

    fn write_8(&mut self, addr: usize, value: u8) -> Result<(), Box<Self::Fault>>;
    fn write_16(&mut self, addr: usize, value: u16) -> Result<(), Box<Self::Fault>>;
    fn write_32(&mut self, addr: usize, value: u32) -> Result<(), Box<Self::Fault>>;
    fn write_64(&mut self, addr: usize, value: u64) -> Result<(), Box<Self::Fault>>;
    fn write_128(&mut self, addr: usize, value: u128) -> Result<(), Box<Self::Fault>>;
    fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), Box<Self::Fault>>;
}

impl BusAccess for Bus {
    type Fault = BusFault;

    #[inline(always)]
    fn read_8(&mut self, addr: usize) -> Result<u8, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 1)?;
        Ok(m.read_8(addr))
    }

    #[inline(always)]
    fn read_16(&mut self, addr: usize) -> Result<u16, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 2)?;
        Ok(m.read_16(addr))
    }

    #[inline(always)]
    fn read_32(&mut self, addr: usize) -> Result<u32, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 4)?;
        Ok(m.read_32(addr))
    }

    #[inline(always)]
    fn read_64(&mut self, addr: usize) -> Result<u64, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        Ok(m.read_64(addr))
    }

    #[inline(always)]
    fn read_128(&mut self, addr: usize) -> Result<u128, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.read_128(addr))
    }

    #[inline(always)]
    fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.read_bytes(addr, buf))
    }

    #[inline(always)]
    fn write_8(&mut self, addr: usize, value: u8) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 1)?;
        Ok(m.write_8(addr, value))
    }

    #[inline(always)]
    fn write_16(&mut self, addr: usize, value: u16) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 2)?;
        Ok(m.write_16(addr, value))
    }

    #[inline(always)]
    fn write_32(&mut self, addr: usize, value: u32) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 4)?;
        Ok(m.write_32(addr, value))
    }

    #[inline(always)]
    fn write_64(&mut self, addr: usize, value: u64) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 8)?;
        Ok(m.write_64(addr, value))
    }

    #[inline(always)]
    fn write_128(&mut self, addr: usize, value: u128) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + 16)?;
        Ok(m.write_128(addr, value))
    }

    #[inline(always)]
    fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr..addr + buf.len())?;
        Ok(m.write_bytes(addr, buf))
    }
}
