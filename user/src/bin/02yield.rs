#![no_std]
#![no_main]

use user_lib::yield_;

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("try to yield in U Mode");
    yield_();
    println!("after yield");
    0
}
