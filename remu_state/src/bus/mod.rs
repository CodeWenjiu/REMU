remu_macro::mod_flat!(memory, device);

pub use memory::{MemFault, MemRegionSpec};

// Use the public re-export to avoid shadowing the glob re-exported `Memory`

#[derive(clap::Args, Debug)]
pub struct BusOption {
    #[arg(
        long = "mem",
        value_name = "NAME@BASE:SIZE",
        action = clap::ArgAction::Append,
        default_value = "ram@0x8000_0000:0x0800_0000"
    )]
    pub mem: Vec<MemRegionSpec>,
}

pub struct Bus {
    memory: Vec<Memory>,
    /// Last-hit cache for region lookup.
    ///
    /// This is an extremely effective fast-path when workloads exhibit any locality
    /// (or when there is only a single region): we first check the previously-hit
    /// region and fall back to scanning only if it doesn't match.
    last_hit: Option<usize>,
}

impl Bus {
    pub(crate) fn new(opt: BusOption) -> Self {
        let memory: Vec<Memory> = opt
            .mem
            .into_iter()
            .map(|region| {
                Memory::new(region)
                    .expect("invalid memory region spec (should be validated before Bus::new)")
            })
            .collect();

        Self {
            memory,
            last_hit: None,
        }
    }

    #[inline(always)]
    fn find_memory_mut(&mut self, addr: usize) -> Result<&mut Memory, Box<MemFault>> {
        // Fast path: check last hit first.
        if let Some(i) = self.last_hit {
            // If memory regions can ever be removed/shrunk, this must be revisited.
            // In current design, regions are built once and remain stable.
            if i < self.memory.len() && self.memory[i].contains(addr) {
                return Ok(&mut self.memory[i]);
            }
        }

        // Slow path: scan all regions.
        // First match wins. If you later allow overlapping regions, you must define priority.
        // For now, regions are expected to be non-overlapping.
        for (i, m) in self.memory.iter_mut().enumerate() {
            if m.contains(addr) {
                self.last_hit = Some(i);
                return Ok(m);
            }
        }

        // If address isn't mapped, keep last_hit unchanged (it may still be useful).
        Err(Box::new(MemFault::Unmapped { addr }))
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
    type Fault = MemFault;

    #[inline(always)]
    fn read_8(&mut self, addr: usize) -> Result<u8, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.read_8(addr - m.base)
    }

    #[inline(always)]
    fn read_16(&mut self, addr: usize) -> Result<u16, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.read_16(addr - m.base)
    }

    #[inline(always)]
    fn read_32(&mut self, addr: usize) -> Result<u32, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.read_32(addr - m.base)
    }

    #[inline(always)]
    fn read_64(&mut self, addr: usize) -> Result<u64, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.read_64(addr - m.base)
    }

    #[inline(always)]
    fn read_128(&mut self, addr: usize) -> Result<u128, Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.read_128(addr - m.base)
    }

    #[inline(always)]
    fn read_bytes(&mut self, addr: usize, buf: &mut [u8]) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.read_bytes(addr - m.base, buf)
    }

    #[inline(always)]
    fn write_8(&mut self, addr: usize, value: u8) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.write_8(addr - m.base, value)
    }

    #[inline(always)]
    fn write_16(&mut self, addr: usize, value: u16) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.write_16(addr - m.base, value)
    }

    #[inline(always)]
    fn write_32(&mut self, addr: usize, value: u32) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.write_32(addr - m.base, value)
    }

    #[inline(always)]
    fn write_64(&mut self, addr: usize, value: u64) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.write_64(addr - m.base, value)
    }

    #[inline(always)]
    fn write_128(&mut self, addr: usize, value: u128) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.write_128(addr - m.base, value)
    }

    #[inline(always)]
    fn write_bytes(&mut self, addr: usize, buf: &[u8]) -> Result<(), Box<Self::Fault>> {
        let m = self.find_memory_mut(addr)?;
        m.write_bytes(addr - m.base, buf)
    }
}
