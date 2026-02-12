use object::{Object as _, ObjectSegment as _};

use super::entry::MemoryEntry;

/// Best-effort load an ELF file into the given memory entries. ELF can only be loaded into
/// RAM (memory entries), never into devices.
pub fn try_load_elf_into_memory(
    memory: &mut [MemoryEntry],
    elf: &Option<std::path::PathBuf>,
    tracer: &remu_types::TracerDyn,
) {
    let Some(path) = elf.as_ref() else {
        return;
    };

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

    for seg in obj.segments() {
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

    tracing::info!(
        "Loaded ELF into memory region '{}' at [{:#x}:{:#x}) from {}",
        memory[i].name,
        start,
        end,
        path.display()
    );
}
