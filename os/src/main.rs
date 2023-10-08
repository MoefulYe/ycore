#![feature(panic_info_message)]
#![no_main]
#![no_std]

#[macro_use]
mod console;
mod constant;
mod lang_items;
mod loader;
mod logging;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

use crate::{loader::Loader, sbi::shutdown};
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
        let num_app = Loader::load_apps();
        Scheduler::init(num_app);
        info!("[kernel] Welcome to Coelophysis TimeSharingPreemptiveOS!");
        show_mem_layout();
        timer::init();
    }
}

fn show_mem_layout() {
    extern "C" {
        fn stext();
        fn etext();
        fn srodata();
        fn erodata();
        fn sdata();
        fn edata();
        fn sbss();
        fn ebss();
        fn boot_stack_lower_bound();
        fn boot_stack_top();
    }
    trace!("[kernel] Mem layout:");
    trace!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize,
        etext as usize
    );
    trace!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize,
        erodata as usize
    );
    trace!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize,
        edata as usize
    );
    trace!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    trace!(
        "[kernel] boot_stack [{:#x}, {:#x})",
        boot_stack_lower_bound as usize,
        boot_stack_top as usize
    );
}
