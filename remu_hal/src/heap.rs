//! Global heap allocator for remu_hal.
//!
//! Call [`init`] at the start of `main` before using any heap-allocated types (Vec, Box, etc.).

use core::alloc::Layout;
use embedded_alloc::LlffHeap as Heap;

unsafe extern "C" {
    static mut __sheap: u8;
    static mut __eheap: u8;
}

#[global_allocator]
static HEAP: Heap = Heap::empty();

/// Must be called once at the start of main, before any heap allocation.
///
/// # Safety
///
/// Must only be called once. The linker symbols __sheap and __eheap must be valid.
pub unsafe fn init() {
    let heap_start = core::ptr::addr_of_mut!(__sheap) as usize;
    let heap_end = core::ptr::addr_of_mut!(__eheap) as usize;
    let size = heap_end.saturating_sub(heap_start);
    if size > 0 {
        unsafe { HEAP.init(heap_start, size) };
    }
}

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    panic!("alloc error: {:?}", layout);
}
