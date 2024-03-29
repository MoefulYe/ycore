mod fs;
mod process;
pub mod signal;

pub use errorno::*;
use fs::*;
use process::*;

use crate::{
    syscall::signal::{sys_kill, sys_sigaction, sys_sigprocmask, sys_sigret},
    types::CStr,
};

pub mod syscall_id {
    pub const DUP: usize = 24;
    pub const OPEN: usize = 56;
    pub const CLOSE: usize = 57;
    pub const PIPE: usize = 59;
    pub const SEEK: usize = 62;
    pub const READ: usize = 63;
    pub const WRITE: usize = 64;
    pub const EXIT: usize = 93;
    pub const YIELD: usize = 124;
    pub const KILL: usize = 129;
    pub const SIGACTION: usize = 134;
    pub const SIGPROCMASK: usize = 135;
    pub const SIGRET: usize = 139;
    pub const GET_TIME: usize = 169;
    pub const GETPID: usize = 172;
    pub const SBRK: usize = 214;
    pub const FORK: usize = 220;
    pub const EXEC: usize = 221;
    pub const WAITPID: usize = 260;
}

#[allow(unused)]
pub mod errorno {
    pub const EOF: isize = 0;
    pub const UNREADABLE: isize = -2;
    pub const UNWRITABLE: isize = -3;
    pub const UNSEEKABLE: isize = -4;
    pub const SEEK_OUT_OF_RANGE: isize = -5;
    pub const PIPE_READER_CLOSED: isize = -6;
}

pub fn syscall(id: usize, [arg0, arg1, arg2]: [usize; 3]) -> isize {
    use syscall_id::*;
    match id {
        DUP => sys_dup(arg0),
        OPEN => sys_open(arg0 as CStr, arg1),
        CLOSE => sys_close(arg0),
        PIPE => sys_pipe(arg0 as *mut _),
        SEEK => sys_seek(arg0, arg1 as isize, arg2),
        READ => sys_read(arg0, arg1, arg2),
        WRITE => sys_write(arg0, arg1, arg2),
        EXIT => sys_exit(arg0 as i32),
        YIELD => sys_yield(),
        KILL => sys_kill(arg0, arg1),
        SIGACTION => sys_sigaction(arg0, arg1, arg2),
        SIGPROCMASK => sys_sigprocmask(arg0),
        SIGRET => sys_sigret(),
        GET_TIME => sys_get_time(),
        SBRK => sys_sbrk(arg0 as isize),
        GETPID => sys_getpid(),
        WAITPID => sys_wait(arg0 as isize, arg1 as *mut i32),
        FORK => sys_fork(),
        EXEC => sys_exec(arg0 as CStr, arg1 as *const CStr),
        _ => panic!("unsupported syscall id {}", id),
    }
}
