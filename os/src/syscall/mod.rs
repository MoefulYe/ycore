mod fs;
mod process;

pub mod syscall_id {
    pub const WRITE: usize = 64;
    pub const EXIT: usize = 93;
}

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
    use syscall_id::*;
    match id {
        WRITE => fs::sys_write(args[0], args[1] as *const u8, args[2]),
        EXIT => process::sys_exit(args[0] as i32),
        _ => panic!("unsupported syscall id {}", id),
    }
}
