use log::debug;

pub mod address;
pub mod frame_alloc;
pub mod heap_alloc;
pub mod kernel_layout;
pub mod kernel_stack;
pub mod mem_set;
pub mod page_table;
pub mod virt_mem_area;

pub fn init() {
    debug!("[heap-allocator] init heap allocator");
    heap_alloc::init();
    debug!("[kernel] activate virtual mode");
    mem_set::KERNEL_MEM_SPACE.exclusive_access().activate();
    debug!("1")
}
