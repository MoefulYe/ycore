#![no_std]
#![no_main]

#[macro_use]
extern crate ylib;

use ylib::{exit, fork, wait};

const MAX_CHILD: usize = 30;

#[no_mangle]
pub fn main() -> i32 {
    for i in 0..MAX_CHILD {
        match fork() {
            ylib::ForkResult::Parent(child) => {
                println!("forked child pid = {}", child);
            }
            ylib::ForkResult::Child => {
                println!("child {} is running", i);
                exit(0)
            }
        }
    }
    for _ in 0..MAX_CHILD {
        let (pid, exit_code) = wait();
        println!("child {} exited with code {}", pid, exit_code);
    }
    println!("forktest pass.");
    0
}
