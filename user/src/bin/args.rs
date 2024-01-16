#![no_std]
#![no_main]
extern crate alloc;

use ylib::{println, types::Argv};

#[no_mangle]
fn main(argv: &Argv) -> i32 {
    for (idx, &arg) in argv.iter().enumerate() {
        println!("{}: {}", idx, arg);
    }
    0
}
