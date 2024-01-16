#![no_std]
#![no_main]

#[macro_use]
extern crate ylib;

use ylib::{fclose, fopen, fread, fwrite, OpenFlags};

#[no_mangle]
pub fn main() -> i32 {
    let test_str = "Hello, world!";
    let filea = "filea\0";
    let fd = fopen(
        filea.as_ptr(),
        OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNC,
    )
    .unwrap();
    fwrite(fd, test_str.as_bytes()).unwrap();
    fclose(fd).unwrap();

    let fd = fopen(filea.as_ptr(), OpenFlags::READ).unwrap();
    let mut buffer = [0u8; 100];
    let read_len = fread(fd, &mut buffer).unwrap();
    fclose(fd).unwrap();
    assert_eq!(test_str, core::str::from_utf8(&buffer[..read_len]).unwrap(),);
    println!("file_test passed!");
    0
}
