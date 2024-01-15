#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

extern crate alloc;

#[macro_use]
pub mod console;
pub mod heap_alloc;
mod lang_items;
pub mod types;
pub mod ylib;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    unsafe { heap_alloc::init() };
    ylib::exit(main());
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}
