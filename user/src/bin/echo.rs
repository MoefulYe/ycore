#![no_std]
#![no_main]
extern crate alloc;

use ylib::{println, types::Argv};

#[no_mangle]
fn main(argv: &Argv) -> i32 {
    let out = argv[1..].join(" ");
    println!("{}", out);
    0
}
