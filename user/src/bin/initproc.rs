#![no_std]
#![no_main]

use user_lib::{exec, fork, wait, yield_};
#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    if fork() == 0 {
        exec("ysh\0");
    } else {
        loop {
            let mut exit_code = 0;
            let pid = wait(&mut exit_code);
            if pid == -1 {
                yield_();
                continue;
            }
            println!(
                "\u{1B}[96m[initproc] Released a zombie process, pid={}, exit_code={}\u{1B}[0m",
                pid, exit_code,
            );
        }
    }
    0
}
