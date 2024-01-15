#![no_std]
#![no_main]
extern crate alloc;

use alloc::string::String;
use user_lib::{fclose, fopen, fread, println, types::Argv, OpenFlags};

#[no_mangle]
fn main(argv: &Argv) -> i32 {
    assert!(argv.len() == 2);
    let fd = fopen(argv[1].as_ptr() as *const _, OpenFlags::READ).expect("cat: open failed");
    let mut buf = [0u8; 128];
    let mut res = String::new();
    loop {
        let read = fread(fd, &mut buf).expect("cat: read failed");
        if read == 0 {
            break;
        }
        res.push_str(core::str::from_utf8(&buf[..read]).expect("cat: invalid utf8"));
    }
    println!("{}", res);
    fclose(fd).expect("cat: close failed");
    0
}
