use crate::process::{pid::task_find, processor::PROCESSOR, signal::SignalFlags};

pub fn sys_kill(pid: usize, signal: usize) -> isize {
    if let (Some(task), Some(signal)) = (task_find(pid), SignalFlags::from_bits(signal as i32)) {
        let task = unsafe { &mut *task };
        if task.signals.contains(signal) {
            return -1;
        }
        task.signals.insert(signal);
        0
    } else {
        -1
    }
}

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

pub fn sys_sigaction(signal: usize, new_action: usize, old_action: usize) -> isize {
    if SignalFlags::from_bits_truncate(1 << signal as i32)
        .intersects(SignalFlags::SIGKILL | SignalFlags::SIGSTOP)
    {
        return -1;
    }

    let task = PROCESSOR.exclusive_access().current().unwrap();
    let page_table = task.page_table();
    let action = match task.signal_actions.get_mut(signal) {
        Some(action) => action,
        None => return -1,
    };
    if old_action != 0 {
        *page_table.translate_virt_mut(old_action as *mut _) = *action;
    }
    if new_action != 0 {
        *action = *page_table.translate_virt_ref(new_action as *const _);
    }
    0
}

pub fn sys_sigret() -> isize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    task.handling_sig = None;
    let trap_ctx = task.trap_ctx();
    *trap_ctx = task.trap_ctx_backup;
    0
}
