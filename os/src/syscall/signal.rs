use crate::process::{processor::PROCESSOR, signal::SignalFlags};

pub fn sys_sigprocmask(mask: usize) -> isize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    let old = task.signal_mask;
    if let Some(mask) = SignalFlags::from_bits(mask as i32) {
        task.signal_mask = mask;
        old.bits() as isize
    } else {
        -1
    }
}

pub fn sys_sigaction(signal: usize, action: usize, old_action: usize) -> isize {
    todo!()
}
