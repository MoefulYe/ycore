#![no_std]
#![no_main]

use core::ptr::null;

use user_lib::{
    exec, fork, println, wait, yield_,
    ForkResult::{Child, Parent},
};

extern crate user_lib;

fn recycle() -> ! {
    loop {
        match wait() {
            Ok((pid, code)) => {
                println!("child {} exited with code {}", pid, code);
            }
            Err(_) => {
                yield_();
                continue;
            }
        }
    }
}

#[no_mangle]
fn main(_: &[&'static str]) -> i32 {
    match fork() {
        Parent(_) => recycle(),
        Child => exec("ysh\0", &[null()]),
    }
}
