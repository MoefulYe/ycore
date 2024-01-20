#![no_std]
#![no_main]

use ylib::{
    getpid, kill, println, sig_ret, sig_setaction, types::Argv, SignalAction, SignalFlags, SIGUSR2,
};

fn action() -> ! {
    println!("from signal handler");
    sig_ret();
}

#[no_mangle]
fn main(_: &Argv) -> i32 {
    let pid = getpid();
    let action = SignalAction::new(action, SignalFlags::empty());
    sig_setaction(SIGUSR2, action);
    kill(pid, SIGUSR2).unwrap();
    println!("hello world");
    0
}
