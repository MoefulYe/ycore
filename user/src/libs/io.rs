use crate::syscall::{sys_close, sys_dup, sys_open, sys_pipe, sys_read, sys_seek, sys_write};

use super::types::{CStr, Fd, Result};
use bitflags::bitflags;

pub fn fdup(fd: usize) -> Result<Fd> {
    let ret = sys_dup(fd);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as Fd)
    }
}

bitflags! {
    pub struct OpenFlags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const CREATE = 1 << 2;
        const APPEND = 1 << 3;
        const TRUNC = 1 << 4;
    }
}

pub fn fopen(path: CStr, flags: OpenFlags) -> Result<Fd> {
    let ret = sys_open(path as usize, flags.bits() as usize);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as Fd)
    }
}

pub fn fclose(fd: Fd) -> Result<()> {
    let ret = sys_close(fd);
    if ret != 0 {
        Err(())
    } else {
        Ok(())
    }
}

pub enum SeekType {
    Set = 0,
    Cur = 1,
    End = 2,
}

pub fn fseek(fd: Fd, offset: isize, whence: SeekType) -> Result<usize> {
    let ret = sys_seek(fd, offset as usize, whence as usize);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

pub fn fread(fd: Fd, buffer: &mut [u8]) -> Result<usize> {
    let ret = sys_read(fd, buffer.as_mut_ptr() as usize, buffer.len());
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

pub fn fwrite(fd: Fd, buffer: &[u8]) -> Result<usize> {
    let ret = sys_write(fd, buffer.as_ptr() as usize, buffer.len());
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

pub fn make_pipe() -> Result<[Fd; 2]> {
    let mut pipe: [Fd; 2] = [0, 0];
    let ret = sys_pipe(&mut pipe as *mut _ as usize);
    if ret < 0 {
        Err(())
    } else {
        Ok(pipe)
    }
}
