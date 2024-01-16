#![no_std]
#![no_main]

use user_lib::shutdown;
use user_lib::types::Argv;

#[no_mangle]
fn main(_: &Argv) -> i32 {
    shutdown();
}
