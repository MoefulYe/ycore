#![feature(panic_info_message)]
#![no_main]
#![no_std]

#[macro_use]
mod console;
mod lang_items;
mod logging;
mod sbi;

use crate::sbi::shutdown;
use core::arch::global_asm;
use log::*;

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    init();
    shutdown();
}

fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) })
}

fn init() {
    clear_bss();
    logging::init();
    info!("[kernel] Welcome to TrilobiteOS!");
    show_mem_layout();
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
    info!("[kernel] Mem layout:");
    info!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize, etext as usize
    );
    info!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
    info!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
    info!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    info!(
        "[kernel] boot_stack [{:#x}, {:#x})",
        boot_stack_lower_bound as usize, boot_stack_top as usize
    );
}
