#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(slice_from_ptr_range)]

extern crate alloc;
use alloc::vec::Vec;
use core::{slice, str};
use types::CStr;
pub use ylib::*;

#[macro_use]
pub mod console;
pub mod heap_alloc;
mod lang_items;
pub mod types;
pub mod ylib;

#[no_mangle]
#[link_section = ".text.entry"]
pub unsafe extern "C" fn _start(argc: usize, argv_base: *const CStr) -> ! {
    heap_alloc::init();
    let mut argv: Vec<&'static str> = Vec::with_capacity(argc);
    for i in 0..argc {
        let arg_base = (argv_base.add(i)).read_volatile();
        let arg_end = (arg_base as usize..)
            .find(|&x| (x as *const u8).read_volatile() == 0)
            .unwrap_unchecked();
        argv.push(str::from_utf8_unchecked(slice::from_ptr_range(
            arg_base..arg_end as *const u8,
        )));
    }

    exit(main(&argv));
}

#[linkage = "weak"]
#[no_mangle]
fn main(_: &[&'static str]) -> i32 {
    panic!("Cannot find main!");
}
