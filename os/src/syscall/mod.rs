use log::debug;

mod fs;
mod process;

use fs::*;
use process::*;

pub mod syscall_id {
    pub const WRITE: usize = 64;
    pub const EXIT: usize = 93;
    pub const YIELD: usize = 124;
    pub const GET_TIME: usize = 169;
    pub const GETPID: usize = 172;
    pub const SBRK: usize = 214;
    pub const FORK: usize = 220;
    pub const EXEC: usize = 221;
    pub const WAITPID: usize = 260;
}

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    use syscall_id::*;
    debug!(
        "syscall: id: {}, args: [{:#x}, {:#x}, {:#x}]",
        id, args[0], args[1], args[2]
    );
    match id {
        WRITE => sys_write(args[0], args[1], args[2]),
        EXIT => sys_exit(args[0] as i32),
        YIELD => sys_yield(),
        GET_TIME => sys_get_time(),
        SBRK => sys_sbrk(args[0] as isize),
        GETPID => sys_getpid(),
        WAITPID => sys_wait(args[0] as isize, args[1] as *mut i32),
        FORK => sys_fork(),
        EXEC => sys_exec(args[0] as *const u8),
        _ => panic!("unsupported syscall id {}", id),
    }
}
