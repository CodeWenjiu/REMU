remu_macro::mod_flat!(
    error, command, option, parse, memory, access, observer, elf_loader
);

remu_macro::mod_pub!(device);

use std::{marker::PhantomData, ops::Range};

pub use memory::MemRegionSpec;
pub use observer::ObserverEvent;
use remu_types::{AllUsize, DynDiagError, isa::RvIsa};

use crate::bus::device::{DeviceAccess, get_device};

pub struct Bus<I: RvIsa, O: BusObserver> {
    memory: Box<[Memory]>,
    device: Box<[(usize, Box<dyn DeviceAccess>)]>,
    last_hit: Option<usize>,
    tracer: remu_types::TracerDyn,
    observer: O,
    _marker: PhantomData<I>,
}

impl<I: RvIsa, O: BusObserver> Bus<I, O> {
    pub(crate) fn new(opt: BusOption, tracer: remu_types::TracerDyn, is_dut: bool) -> Self {
        let prefix = if is_dut { "[DUT]" } else { "[REF]" };
        let memory: Vec<Memory> = opt
            .mem
            .into_iter()
            .map(|region| {
                tracing::info!(
                    "{} new memory {} region initialized at 0x{:08x}:0x{:08x}",
                    prefix,
                    region.name,
                    region.region.start,
                    region.region.end
                );
                Memory::new(region)
                    .expect("invalid memory region spec (should be validated before Bus::new)")
            })
            .collect();

        let mut memory = memory.into_boxed_slice();

        try_load_elf_into_memory(&mut memory, &opt.elf, &tracer);

        let device: Vec<(usize, Box<dyn DeviceAccess>)> = if is_dut {
            opt.devices
                .iter()
                .map(|config| {
                    tracing::info!(
                        "{} new device {} config initialized at 0x{:08x}",
                        prefix,
                        config.name,
                        config.start
                    );
                    (
                        config.start,
                        get_device(&config.name).expect("invalid device name"),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };

        Self {
            memory,
            device: device.into_boxed_slice(),
            last_hit: None,
            tracer,
            observer: O::new(),
            _marker: PhantomData,
        }
    }

    /// 取回并清空本步的 observer 事件（上层根据事件类型决定行为，如 difftest 时 MmioiAccess 则 ref 不 step 而 sync）。
    #[inline(always)]
    pub fn take_observer_event(&mut self) -> observer::ObserverEvent {
        self.observer.get_enent_and_clear()
    }

    #[inline(always)]
    fn find_memory_mut(&mut self, range: Range<usize>) -> Option<&mut Memory> {
        // Fast path: check last hit first.
        if let Some(i) = self.last_hit {
            // SAFETY: i must be a valid index into self.memory.
            if unsafe { self.memory.get_unchecked(i).contains(range.clone()) } {
                return Some(&mut self.memory[i]);
            }
        }

        // Slow path: scan all regions.
        self.find_memory_mut_slow(range)
    }

    #[inline(always)]
    fn find_memory_mut_slow(&mut self, range: Range<usize>) -> Option<&mut Memory> {
        // First match wins. If you later allow overlapping regions, you must define priority.
        // For now, regions are expected to be non-overlapping.
        for (i, m) in self.memory.iter_mut().enumerate() {
            if m.contains(range.clone()) {
                self.last_hit = Some(i);
                return Some(m);
            }
        }

        None
    }

    fn find_device_mut(
        &mut self,
        range: Range<usize>,
    ) -> Option<(usize, &mut Box<dyn DeviceAccess>)> {
        for (addr, device) in self.device.iter_mut() {
            if addr == &range.start {
                return Some((*addr, device));
            }
        }

        None
    }

    pub(crate) fn execute(&mut self, subcmd: &BusCmd) -> Result<(), BusError> {
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
            BusCmd::Write { subcmd } => match subcmd {
                WriteCommand::U8 { addr, value } => self.write_8(*addr, *value)?,
                WriteCommand::U16 { addr, value } => self.write_16(*addr, *value)?,
                WriteCommand::U32 { addr, value } => self.write_32(*addr, *value)?,
                WriteCommand::U64 { addr, value } => self.write_64(*addr, *value)?,
                WriteCommand::U128 { addr, value } => self.write_128(*addr, *value)?,
            },
            BusCmd::Set { address, value } => {
                let mut addr = *address;
                for chunk in value.iter() {
                    if chunk.is_empty() {
                        continue;
                    }
                    self.write_bytes(addr, chunk)?;
                    addr = addr.saturating_add(chunk.len());
                }
            }
            BusCmd::MemMap => {
                self.tracer.borrow().mem_show_map(
                    self.memory
                        .iter()
                        .map(|m| (m.name.clone(), m.range.clone()))
                        .chain(
                            self.device
                                .iter()
                                .map(|d| (d.1.name().to_string(), d.0..d.0 + d.1.size())),
                        )
                        .collect(),
                );
            }
        }
        Ok(())
    }
}
