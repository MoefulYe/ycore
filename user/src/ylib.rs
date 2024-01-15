use crate::types::{CStr, ExitCode, Fd, Ms, Pid};
use bitflags::bitflags;
use core::arch::asm;

const SYSCALL_DUP: usize = 24;
const SYSCALL_OPEN: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE: usize = 59;
const SYSCALL_SEEK: usize = 62;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_SBRK: usize = 214;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;

type Result<T = (), E = ()> = core::result::Result<T, E>;

fn syscall(id: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x17") id
        );
    }
    ret
}

fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}

pub fn fdup(fd: usize) -> Result<Fd> {
    let ret = sys_dup(fd);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as Fd)
    }
}

fn sys_open(path: CStr, flags: u32) -> isize {
    syscall(SYSCALL_OPEN, [path as usize, flags as usize, 0])
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
    let ret = sys_open(path, flags.bits());
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as Fd)
    }
}

fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

pub fn fclose(fd: Fd) -> Result<()> {
    let ret = sys_close(fd);
    if ret != 0 {
        Err(())
    } else {
        Ok(())
    }
}

fn sys_seek(fd: usize, offset: isize, whence: usize) -> isize {
    syscall(SYSCALL_SEEK, [fd, offset as usize, whence])
}

pub enum SeekType {
    Set = 0,
    Cur = 1,
    End = 2,
}

pub fn fseek(fd: Fd, offset: isize, whence: SeekType) -> Result<usize> {
    let ret = sys_seek(fd, offset, whence as usize);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYSCALL_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

pub fn fread(fd: Fd, buffer: &mut [u8]) -> Result<usize> {
    let ret = sys_read(fd, buffer);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn fwrite(fd: Fd, buffer: &[u8]) -> Result<usize> {
    let ret = sys_write(fd, buffer);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

fn sys_exit(exit_code: i32) -> isize {
    syscall(SYSCALL_EXIT, [exit_code as usize, 0, 0])
}

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code);
    panic!("unreachable after sys_exit!");
}

fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

pub fn yield_() {
    sys_yield();
}

fn sys_get_time() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

pub fn time() -> Ms {
    sys_get_time() as Ms
}

fn sys_sbrk(size: isize) -> isize {
    syscall(SYSCALL_SBRK, [size as usize, 0, 0])
}

pub fn sbrk(size: isize) -> Result<isize> {
    let ret = sys_sbrk(size);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret)
    }
}

//对于父进程，fork 返回新创建子进程的进程 ID；
//对于子进程，fork 返回 0；
fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

// 枚举区分父子进程
pub enum ForkResult {
    Parent(Pid),
    Child,
}

pub fn fork() -> ForkResult {
    let ret = sys_fork();
    if ret == 0 {
        ForkResult::Child
    } else {
        ForkResult::Parent(ret as Pid)
    }
}

// -1 表示出错，否则表示成功执行
fn sys_exec(path: &str, args: &[CStr]) -> isize {
    syscall(
        SYSCALL_EXEC,
        [path.as_ptr() as usize, args.as_ptr() as usize, 0],
    )
}

pub fn exec(path: &str, args: &[CStr]) -> ! {
    sys_exec(path, args);
    panic!("unreachable after sys_exec!");
}

// exit_code == NULL 时不必保存
// pid -1 时等待任意子进程退出
// 返回 -1 要等待的子进程不存在
//      -2 要等待的子进程未结束
fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize {
    syscall(SYSCALL_WAITPID, [pid as usize, exit_code as usize, 0])
}

pub enum WaitError {
    NotSuchChild,
    NotExitedYet,
}

pub fn waitpid(pid: Pid) -> Result<(Pid, ExitCode), WaitError> {
    let mut exit_code: i32 = 0;
    let ret = sys_waitpid(pid as isize, &mut exit_code);
    if ret == -1 {
        Err(WaitError::NotSuchChild)
    } else if ret == -2 {
        Err(WaitError::NotExitedYet)
    } else {
        Ok((ret as usize, exit_code))
    }
}

pub fn wait() -> Result<(Pid, ExitCode), WaitError> {
    const ANY: isize = -1;

    let mut exit_code: i32 = 0;
    let ret = sys_waitpid(ANY, &mut exit_code);
    if ret == -1 {
        Err(WaitError::NotSuchChild)
    } else if ret == -2 {
        Err(WaitError::NotExitedYet)
    } else {
        Ok((ret as usize, exit_code))
    }
}

fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

pub fn getpid() -> Pid {
    sys_getpid() as Pid
}

fn sys_pipe(pipe: *mut usize) -> isize {
    syscall(SYSCALL_PIPE, [pipe as usize, 0, 0])
}

pub fn make_pipe() -> Result<[Fd; 2]> {
    let mut pipe: [Fd; 2] = [0, 0];
    let ret = sys_pipe(&mut pipe as *mut _ as *mut usize);
    if ret < 0 {
        Err(())
    } else {
        Ok(pipe)
    }
}
