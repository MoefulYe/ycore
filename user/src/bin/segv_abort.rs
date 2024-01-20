#![no_std]
#![no_main]

use ylib::sleep;

#[no_mangle]
fn main() -> i32 {
    unsafe { core::ptr::null_mut::<usize>().read_volatile() };
    0
}
