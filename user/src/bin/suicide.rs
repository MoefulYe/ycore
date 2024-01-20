#![no_std]
#![no_main]

use ylib::{getpid, kill, types::Argv, SIGKILL};

#[no_mangle]
fn main(_: &Argv) -> i32 {
    let pid = getpid();
    kill(pid, SIGKILL).unwrap();
    0
}
