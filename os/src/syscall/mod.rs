use log::debug;

mod fs;
mod process;

pub mod syscall_id {
    pub const WRITE: usize = 64;
    pub const EXIT: usize = 93;
    pub const YIELD: usize = 124;
}

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    use syscall_id::*;
    debug!(
        "syscall: id: {}, args: [{:#x}, {:#x}, {:#x}]",
        id, args[0], args[1], args[2]
    );
    match id {
        WRITE => fs::sys_write(args[0], args[1] as *const u8, args[2]),
        EXIT => process::sys_exit(args[0] as i32),
        YIELD => process::sys_yield(),
        _ => panic!("unsupported syscall id {}", id),
    }
}
