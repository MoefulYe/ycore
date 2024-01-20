#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![no_main]
#![no_std]

#[macro_use]
extern crate bitflags;
extern crate alloc;
#[macro_use]
extern crate lazy_static;

#[macro_use]
mod console;
mod constant;
pub mod drivers;
pub mod fs;
mod lang_items;
mod logging;
mod mm;
mod process;
mod sbi;
pub mod sync;
mod syscall;
mod timer;
mod trap;
pub mod types;

use crate::{
    process::{initproc::INITPROC, pid::task_insert, processor::PROCESSOR, queue::QUEUE},
    sbi::shutdown,
};
use core::arch::global_asm;
use fs::inode::list_apps;
use log::*;

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    init();
    info!("[kernel] Welcome to TroodontidaeOS!");
    QUEUE
        .exclusive_access()
        .push(INITPROC.exclusive_access() as *mut _);
    task_insert(
        INITPROC.exclusive_access().pid,
        INITPROC.exclusive_access() as *mut _,
    );
    PROCESSOR.exclusive_access().run_tasks();
    shutdown(false);
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

fn init() {
    unsafe {
        clear_bss();
        logging::init();
        mm::init();
        trap::init();
        timer::init();
        list_apps();
    }
}
