use alloc::{string::String, vec::Vec};

use crate::{
    fs::inode::{OSInode, OpenFlags},
    mm::page_table::TopLevelEntry,
    process::{
        pid::{task_insert, Pid},
        processor::PROCESSOR,
        queue::QUEUE,
    },
    timer::get_time_ms,
    types::CStr,
};

pub fn sys_exit(code: i32) -> isize {
    PROCESSOR.exclusive_access().exit_current(code).schedule();
    0
}

pub fn sys_yield() -> isize {
    PROCESSOR.exclusive_access().suspend_current().schedule();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_sbrk(size: isize) -> isize {
    PROCESSOR
        .exclusive_access()
        .current()
        .unwrap()
        .change_brk(size) as isize
}

pub fn sys_fork() -> isize {
    let fork = PROCESSOR.exclusive_access().current().unwrap().fork();
    let pid = unsafe { (*fork).pid() };
    unsafe {
        (*fork).trap_ctx().x[10] = 0;
    }
    QUEUE.exclusive_access().push(fork);
    task_insert(pid, fork);
    pid.0 as isize
}

pub fn sys_exec(path: CStr, mut args: *const CStr) -> isize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    let entry = task.page_table();
    let s = entry.translate_virt_str(path);
    if s == "." {
        return -1;
    }

    let mut argv: Vec<String> = Vec::new();
    loop {
        let arg = *entry.translate_virt_ref(args);
        if arg == core::ptr::null() {
            break;
        }
        argv.push(entry.translate_virt_str(arg));
        args = unsafe { args.add(1) };
    }

    if let Some(inode) = OSInode::open(&s, OpenFlags::READ) {
        let data = inode.read_all();
        task.exec(&data, argv);
        0
    } else {
        -1
    }
}

pub fn sys_wait(pid: isize, exit_code: *mut i32) -> isize {
    let task = PROCESSOR.exclusive_access().current().unwrap();
    let pid = Pid(pid as usize);
    //pid不等于-1或者不等于任意一个子进程的pid
    if !task
        .children
        .iter()
        .any(|&p| pid == Pid::ANY || pid == unsafe { &mut *p }.pid())
    {
        return -1;
    }

    if let Some((idx, &child)) = task.children.iter().enumerate().find(|(_, &p)| {
        let p = unsafe { &mut *p };
        p.is_zombie() && (pid == Pid::ANY || pid == p.pid())
    }) {
        unsafe {
            task.children.remove(idx);
            let child_exit_code = (*child).exit_code;
            let pid = (*child).pid();
            core::ptr::drop_in_place(child);
            *TopLevelEntry::from_token(PROCESSOR.exclusive_access().current_token().unwrap())
                .translate_virt_mut(exit_code) = child_exit_code;
            pid.0 as isize
        }
    } else {
        -2
    }
}

pub fn sys_getpid() -> isize {
    PROCESSOR.exclusive_access().current().unwrap().pid().0 as isize
}
