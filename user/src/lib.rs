#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]

#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
    exit(main());
    panic!("unreachable after exit")
}

fn clear_bss() {
    extern "C" {
        fn start_bss();
        fn end_bss();
    }
    (start_bss as usize..end_bss as usize).for_each(|addr| unsafe {
        (addr as *mut u8).write_volatile(0);
    });
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("no main function");
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    crate::syscall::sys_write(fd, buf)
}

pub fn exit(code: i32) -> isize {
    crate::syscall::sys_exit(code as usize)
}
