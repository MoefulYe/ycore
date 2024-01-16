#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::console::STDIN;
use user_lib::types::Argv;
use user_lib::{fopen, fread, OpenFlags};

#[no_mangle]
pub fn main(argv: &Argv) -> i32 {
    let mut buf = [0u8; 256];
    let mut lines = 0usize;
    let mut total_size = 0usize;
    let fd = if argv.len() == 1 {
        STDIN
    } else {
        fopen(argv[1].as_ptr(), OpenFlags::READ).unwrap()
    };
    loop {
        let len = fread(fd, &mut buf).unwrap();
        if len == 0 {
            break;
        }
        total_size += len;
        let string = core::str::from_utf8(&buf[..len]).unwrap();
        lines += string
            .chars()
            .fold(0, |acc, c| acc + if c == '\n' { 1 } else { 0 });
    }
    if total_size > 0 {
        lines += 1;
    }
    println!("{}", lines);
    0
}
