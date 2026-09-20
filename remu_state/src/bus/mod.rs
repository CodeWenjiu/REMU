remu_macro::mod_pub!(crate, device, memory);
remu_macro::mod_pub!(crate, flow);
remu_macro::mod_prv!(error, parse, access, observer);

use std::{marker::PhantomData, ops::Range, sync::Arc};

pub use device::DeviceKind;
pub use error::BusError;
pub use flow::{BusCmd, BusOption, ReadArgs, ReadCommand, WriteCommand};
pub use memory::{
    AccessKind, MemFault, MemRegionSpec, Memory, MemoryEntry, try_load_elf_image,
    try_load_elf_images, write_app_args_to_entries,
};
pub use observer::{BusObserver, DifftestObserver, FastObserver, ObserverEvent};
pub(crate) use parse::parse_usize_allow_hex_underscore;
use remu_isa::AllUsize;
use remu_isa::isa::RvIsa;
use remu_types::DynDiagError;

use crate::bus::device::{
    DeviceAccess, DeviceConfig, DeviceContext, WindowHost, instantiate_device,
};

/// Validate that no memory region overlaps another memory region or a device
/// MMIO range. Called before building `Memory`; panics with a clear message on
/// collision (layout errors are programmer/config errors, not recoverable).
fn validate_layout(specs: &[MemRegionSpec], devices: &[(DeviceConfig, Box<dyn DeviceAccess>)]) {
    // Memory region vs memory region.
    for (i, a) in specs.iter().enumerate() {
        for b in &specs[i + 1..] {
            let overlap = a.region.start < b.region.end && b.region.start < a.region.end;
            assert!(
                !overlap,
                "memory region '{}' [{:#x}..{:#x}) overlaps '{}' [{:#x}..{:#x})",
                a.name, a.region.start, a.region.end, b.name, b.region.start, b.region.end
            );
        }
    }
    // Memory region vs device MMIO.
    for spec in specs {
        for (cfg, dev) in devices {
            let dev_end = cfg.start + dev.size();
            let overlap = spec.region.start < dev_end && cfg.start < spec.region.end;
            assert!(
                !overlap,
                "memory region '{}' [{:#x}..{:#x}) overlaps device '{}' at [{:#x}..{:#x})",
                spec.name,
                spec.region.start,
                spec.region.end,
                dev.name(),
                cfg.start,
                dev_end
            );
        }
    }
}

pub struct Bus<I: RvIsa, O: BusObserver> {
    memory: Memory,
    device: Box<[(DeviceConfig, Box<dyn DeviceAccess>)]>,
    /// Reset PC when a firmware image (`--firmware`) is loaded: its ELF entry.
    /// `None` = no firmware, the caller's `--init-pc` applies unchanged.
    firmware_entry: Option<usize>,
    /// Address of the boot-info block handed to a loaded firmware in `a1`
    /// (RISC-V boot convention: `a0` = hartid). `None` without firmware.
    boot_info_addr: Option<usize>,
    /// Shared window host (if any device needs it). Owned by this Bus; devices
    /// hold clones. Dropped with the Bus, tearing down the window thread.
    window: Option<Arc<WindowHost>>,
    tracer: remu_types::TracerDyn,
    observer: O,
    _marker: PhantomData<I>,
}

impl<I: RvIsa, O: BusObserver> Bus<I, O> {
    pub(crate) fn new(opt: BusOption, tracer: remu_types::TracerDyn, is_dut: bool) -> Self {
        let prefix = if is_dut { "[DUT]" } else { "[REF]" };

        // 1. Instantiate devices first (they may declare extra memory regions).
        //    Resolve the device configs once so we can build a dependency context
        //    (e.g. the shared window host) before injecting it into each device.
        let dev_configs: Vec<(usize, DeviceKind)> = if is_dut {
            opt.resolve_devices()
                .iter()
                .map(|config| (config.start, config.kind))
                .collect()
        } else {
            Vec::new()
        };

        // Build the dependency context: if any device needs the window host,
        // create it (owned by this Bus). Only the DUT bus has devices, so only
        // it can have a window.
        let kinds: Vec<DeviceKind> = dev_configs.iter().map(|(_, k)| *k).collect();
        let window = if DeviceContext::needs_window(&kinds) {
            Some(WindowHost::new())
        } else {
            None
        };
        let mut ctx = DeviceContext::default();
        if let Some(w) = &window {
            ctx.set_window(Arc::clone(w));
        }

        let mut devices: Vec<(DeviceConfig, Box<dyn DeviceAccess>)> = dev_configs
            .into_iter()
            .map(|(start, kind)| {
                tracing::info!(
                    "{} new device {} config initialized at 0x{:08x}",
                    prefix,
                    kind.as_str(),
                    start
                );
                let cfg = DeviceConfig { kind, start };
                (cfg, instantiate_device(kind, &ctx))
            })
            .collect();

        // 2. Collect memory regions: base + user extras + device-declared extras.
        let mut specs: Vec<MemRegionSpec> = opt.resolve_mem_regions();
        for (_, dev) in devices.iter() {
            specs.extend(dev.extra_mem_regions());
        }

        // 3. Validate no overlap before building Memory (memory-memory and
        //    memory-device). Panic with a clear message on collision.
        validate_layout(&specs, &devices);

        // 4. Build Memory from the complete region list.
        let entries: Vec<MemoryEntry> = specs
            .into_iter()
            .map(|region| {
                tracing::info!(
                    "{} new memory {} region initialized at 0x{:08x}:0x{:08x}",
                    prefix,
                    region.name,
                    region.region.start,
                    region.region.end
                );
                MemoryEntry::new(region)
                    .expect("invalid memory region spec (should be validated before Bus::new)")
            })
            .collect();
        let mut memory = Memory::new(entries.into_boxed_slice());
        // Load images in order (firmware first, then the program). The first
        // loaded image's entry becomes the reset PC — `--firmware` needs no
        // address of its own. Columns: firmware entry, program entry.
        let entries = memory.try_load_elf(&opt.images(), &tracer);
        let firmware_entry = match (opt.firmware.is_some(), entries.first().copied().flatten()) {
            (true, Some(entry)) => Some(entry as usize),
            _ => None,
        };
        let program_entry = entries.get(1).copied().flatten().map(|e| e as usize);

        // Write app args to known address (top of RAM - 4 KiB). Shared with
        // reference simulators (e.g. spike) so the ref sees the same payload.
        memory.write_app_args(&opt.app_args);

        // 5. Attach extra memory region pointers to devices that declared them.
        for (_, dev) in devices.iter_mut() {
            for region in dev.extra_mem_regions() {
                let base = region.region.start;
                let size = region.region.end - region.region.start;
                if let Some(entry) = memory
                    .entries_mut()
                    .iter_mut()
                    .find(|e| e.range.start == base && e.range.end - e.range.start == size)
                {
                    let ptr = entry.ptr_at_addr(base);
                    dev.attach_mem_region(base, ptr, size);
                }
            }
        }

        let boot_info_addr = if firmware_entry.is_some() {
            write_boot_info(&mut memory, &devices, program_entry, &tracer)
        } else {
            None
        };

        Self {
            memory,
            device: devices.into_boxed_slice(),
            firmware_entry,
            boot_info_addr,
            window,
            tracer,
            observer: O::new(),
            _marker: PhantomData,
        }
    }

    /// Reset PC contributed by a loaded firmware image (`--firmware`): its ELF
    /// entry, resolved at load time. `None` when no firmware was configured,
    /// in which case the caller's `--init-pc` applies.
    #[inline]
    pub fn firmware_entry(&self) -> Option<usize> {
        self.firmware_entry
    }

    /// Boot-info address passed to a loaded firmware in `a1` (`None` without
    /// firmware). See [`write_boot_info`].
    #[inline]
    pub fn boot_info_addr(&self) -> Option<usize> {
        self.boot_info_addr
    }

    /// Take and clear all observer events this step (MMIO and/or memory writes).
    #[inline(always)]
    pub fn take_observer_events(&mut self) -> Vec<observer::ObserverEvent> {
        self.observer.get_events_and_clear()
    }

    pub fn mem_regions_for_difftest(&mut self) -> Vec<(usize, *mut u8, usize)> {
        self.memory
            .entries_mut()
            .iter_mut()
            .map(|m| m.difftest_raw_region())
            .collect()
    }

    pub fn mem_regions_for_sync(&self) -> Vec<(usize, *const u8, usize)> {
        self.memory
            .entries()
            .iter()
            .map(|m| m.difftest_raw_region_read())
            .collect()
    }

    pub fn write_bytes_at(&mut self, addr: usize, data: &[u8]) -> Result<(), crate::bus::BusError> {
        if self.memory.write_bytes(addr, data).is_some() {
            Ok(())
        } else {
            Err(crate::bus::BusError::unmapped(addr))
        }
    }

    fn find_device_mut(
        &mut self,
        range: Range<usize>,
    ) -> Option<(usize, &mut Box<dyn DeviceAccess>)> {
        for (cfg, device) in self.device.iter_mut() {
            let addr = cfg.start;
            let device_end = addr + device.size();
            if range.start >= addr && range.end <= device_end {
                return Some((addr, device));
            }
        }

        None
    }

    /// Start address of the first device of `kind`, if it is mapped.
    /// Lets the boot-info writer (`--firmware` setup) resolve the device map
    /// without duplicating it; the address comes from the same configuration
    /// the bus was actually built from.
    pub fn device_addr(&self, kind: DeviceKind) -> Option<usize> {
        self.device
            .iter()
            .find(|(cfg, _)| cfg.kind == kind)
            .map(|(cfg, _)| cfg.start)
    }

    pub(crate) fn execute(&mut self, subcmd: &BusCmd) -> Result<(), BusError> {
        match subcmd {
            BusCmd::Read { subcmd } => {
                let (addr, result) = match subcmd {
                    ReadCommand::U8(arg) => (
                        arg.addr,
                        self.read_8_impl::<false>(arg.addr).map(|v| AllUsize::U8(v)),
                    ),
                    ReadCommand::U16(arg) => (
                        arg.addr,
                        self.read_16_impl::<false>(arg.addr)
                            .map(|v| AllUsize::U16(v)),
                    ),
                    ReadCommand::U32(arg) => (
                        arg.addr,
                        self.read_32_impl::<false>(arg.addr)
                            .map(|v| AllUsize::U32(v)),
                    ),
                    ReadCommand::U64(arg) => (
                        arg.addr,
                        self.read_64_impl::<false>(arg.addr)
                            .map(|v| AllUsize::U64(v)),
                    ),
                    ReadCommand::U128(arg) => (
                        arg.addr,
                        self.read_128_impl::<false>(arg.addr)
                            .map(|v| AllUsize::U128(v)),
                    ),
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
                WriteCommand::U8 { addr, value } => self.write_8_impl::<false>(*addr, *value)?,
                WriteCommand::U16 { addr, value } => self.write_16_impl::<false>(*addr, *value)?,
                WriteCommand::U32 { addr, value } => self.write_32_impl::<false>(*addr, *value)?,
                WriteCommand::U64 { addr, value } => self.write_64_impl::<false>(*addr, *value)?,
                WriteCommand::U128 { addr, value } => {
                    self.write_128_impl::<false>(*addr, *value)?
                }
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
                        .entries()
                        .iter()
                        .map(|m| (m.name.clone(), m.range.clone()))
                        .chain(self.device.iter().map(|(cfg, d)| {
                            (d.name().to_string(), cfg.start..cfg.start + d.size())
                        }))
                        .collect(),
                );
            }
        }
        Ok(())
    }
}

impl<I: RvIsa, O: BusObserver> Drop for Bus<I, O> {
    fn drop(&mut self) {
        // Tear down the window host when the bus is dropped: signal the render
        // thread to stop and join it, so the window closes with the bus. This
        // also exercises the `window` field (its lifetime is owned here).
        if let Some(w) = &self.window {
            w.request_shutdown();
        }
    }
}

/// Fixed address of the boot-info block for a loaded firmware (`--firmware`).
/// The firmware reads it via `a1` at reset (RISC-V boot convention) and must
/// use the *same* layout — see `remu_firmware`'s `boot_info` module.
pub(crate) const BOOT_INFO_BASE: usize = 0x87FF_E000;

/// Magic for [`BootInfo`]: ASCII "REMU".
pub(crate) const BOOT_INFO_MAGIC: u32 = 0x5245_4D55;

/// Version of the [`BootInfo`] layout; mismatch makes the firmware stop.
pub(crate) const BOOT_INFO_VERSION: u32 = 1;

/// Boot-info handed to a loaded firmware in `a1`: the resolved device map
/// (addresses come from the user configuration, never hardcoded) and the
/// handover target the firmware jumps to after it finishes.
///
/// Fields are u64 (XLEN-independent so RV32/RV64 firmwares share the layout);
/// the struct is written as raw bytes, so it must stay `repr(C)` and the
/// firmware side must mirror it exactly.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct BootInfo {
    pub(crate) magic: u32,
    pub(crate) version: u32,
    /// UART 16550 base (0 = not configured).
    pub(crate) uart_base: u64,
    /// SiFive test finisher base (0 = not configured).
    pub(crate) finisher_base: u64,
    /// CLINT base (0 = not configured).
    pub(crate) clint_base: u64,
    /// S-mode payload (kernel) entry point.
    pub(crate) kernel_entry: u64,
}

/// Write the [`BootInfo`] block at [`BOOT_INFO_BASE`]; returns its address
/// when the block is inside a mapped RAM region, `None` otherwise.
fn write_boot_info(
    memory: &mut Memory,
    devices: &[(DeviceConfig, Box<dyn DeviceAccess>)],
    kernel_entry: Option<usize>,
    tracer: &remu_types::TracerDyn,
) -> Option<usize> {
    let find_addr = |kind: DeviceKind| {
        devices
            .iter()
            .find(|(cfg, _)| cfg.kind == kind)
            .map(|(cfg, _)| cfg.start as u64)
            .unwrap_or(0)
    };
    let info = BootInfo {
        magic: BOOT_INFO_MAGIC,
        version: BOOT_INFO_VERSION,
        uart_base: find_addr(DeviceKind::Uart16550),
        finisher_base: find_addr(DeviceKind::SifiveTestFinisher),
        clint_base: find_addr(DeviceKind::Clint),
        kernel_entry: kernel_entry.unwrap_or(0) as u64,
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (&info as *const BootInfo).cast::<u8>(),
            core::mem::size_of::<BootInfo>(),
        )
    };
    if memory.write_bytes(BOOT_INFO_BASE, bytes).is_some() {
        tracing::info!("wrote boot info block for firmware at 0x{BOOT_INFO_BASE:x}");
        Some(BOOT_INFO_BASE)
    } else {
        tracer.borrow().print(&format!(
            "firmware loaded but boot-info area 0x{BOOT_INFO_BASE:x} is not in a RAM region; \
             the firmware will not receive the device map"
        ));
        None
    }
}
