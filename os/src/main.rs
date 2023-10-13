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
mod lang_items;
mod loader;
mod logging;
mod mm;
mod sbi;
pub mod sync;
mod syscall;
mod task;
mod timer;
mod trap;

use crate::{loader::Loader, mm::heap_alloc, sbi::shutdown};
use core::arch::global_asm;
use log::*;
use task::Scheduler;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_apps.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    init();
    Scheduler::singletion().run();
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
        trap::init();
        mm::init();
        let num_app = Loader::load_apps();
        Scheduler::init(num_app);
        info!("[kernel] Welcome to CoelophysisOS! (support virtual memory!)");
        timer::init();
    }
}
