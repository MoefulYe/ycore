#![no_std]
#![no_main]

use ylib::shutdown;
use ylib::types::Argv;

#[no_mangle]
fn main(_: &Argv) -> i32 {
    shutdown();
}
