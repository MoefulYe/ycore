#![no_std]
#![no_main]

use core::ptr::null;

use ylib::{
    exec, fork, println, wait,
    ForkResult::{Child, Parent},
};

extern crate ylib;

fn recycle() -> ! {
    loop {
        let (pid, code) = wait();
        println!("initproc: child {} exited with code {}", pid, code);
    }
}

#[no_mangle]
fn main(_: &[&'static str]) -> i32 {
    match fork() {
        Parent(_) => recycle(),
        Child => exec("ysh\0", &[null()]),
    }
}
