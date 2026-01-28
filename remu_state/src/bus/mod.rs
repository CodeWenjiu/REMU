remu_macro::mod_flat!(error, command, option, memory, device, access, elf_loader);

use std::{marker::PhantomData, ops::Range};

pub use memory::MemRegionSpec;
use remu_types::{AllUsize, DynDiagError, isa::RvIsa};

// Use the public re-export to avoid shadowing the glob re-exported `Memory`

pub struct Bus<I: RvIsa> {
    memory: Box<[Memory]>,
    /// Last-hit cache for region lookup.
    ///
    /// This is an extremely effective fast-path when workloads exhibit any locality
    /// (or when there is only a single region): we first check the previously-hit
    /// region and fall back to scanning only if it doesn't match.
    last_hit: Option<usize>,
    tracer: remu_types::TracerDyn,
    _marker: PhantomData<I>,
}

impl<I: RvIsa> Bus<I> {
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
            _marker: PhantomData,
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
