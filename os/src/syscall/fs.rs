use log::debug;

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
pub const STDERR: usize = 2;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    debug!("sys_write: fd: {}, buffer: {:p}, len: {:#x}", fd, buf, len);
    match fd {
        STDOUT => {
            let buffer = unsafe { core::slice::from_raw_parts(buf, len) };
            let s = core::str::from_utf8(buffer).unwrap();
            print!("{}", s);
            len as isize
        }
        _ => panic!("sys_write: fd {fd} not supported!"),
    }
}
