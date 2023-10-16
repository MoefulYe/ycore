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
    pub const SBRK: usize = 214;
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
        _ => panic!("unsupported syscall id {}", id),
    }
}
