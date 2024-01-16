use core::arch::asm;

pub const SYSCALL_DUP: usize = 24;
pub const SYSCALL_OPEN: usize = 56;
pub const SYSCALL_CLOSE: usize = 57;
pub const SYSCALL_PIPE: usize = 59;
pub const SYSCALL_SEEK: usize = 62;
pub const SYSCALL_READ: usize = 63;
pub const SYSCALL_WRITE: usize = 64;
pub const SYSCALL_EXIT: usize = 93;
pub const SYSCALL_YIELD: usize = 124;
pub const SYSCALL_KILL: usize = 129;
pub const SYSCALL_GET_TIME: usize = 169;
pub const SYSCALL_GETPID: usize = 172;
pub const SYSCALL_SBRK: usize = 214;
pub const SYSCALL_FORK: usize = 220;
pub const SYSCALL_EXEC: usize = 221;
pub const SYSCALL_WAITPID: usize = 260;

pub fn syscall(id: usize, args: [usize; 3]) -> isize {
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

pub fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd, 0, 0])
}

pub fn sys_open(path: usize, flags: usize) -> isize {
    syscall(SYSCALL_OPEN, [path, flags, 0])
}

pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd, 0, 0])
}

pub fn sys_seek(fd: usize, offset: usize, whence: usize) -> isize {
    syscall(SYSCALL_SEEK, [fd, offset, whence])
}

pub fn sys_read(fd: usize, buffer: usize, len: usize) -> isize {
    syscall(SYSCALL_READ, [fd, buffer, len])
}

pub fn sys_write(fd: usize, buffer: usize, len: usize) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer, len])
}

pub fn sys_exit(exit_code: usize) -> isize {
    syscall(SYSCALL_EXIT, [exit_code, 0, 0])
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

pub fn sys_kill(pid: usize, signal: usize) -> isize {
    syscall(SYSCALL_KILL, [pid, signal, 0])
}

pub fn sys_gettime() -> isize {
    syscall(SYSCALL_GET_TIME, [0, 0, 0])
}

pub fn sys_sbrk(size: usize) -> isize {
    syscall(SYSCALL_SBRK, [size, 0, 0])
}

pub fn sys_fork() -> isize {
    syscall(SYSCALL_FORK, [0, 0, 0])
}

pub fn sys_exec(path: usize, args: usize) -> isize {
    syscall(SYSCALL_EXEC, [path, args, 0])
}

// exit_code == NULL 时不必保存
// pid -1 时等待任意子进程退出
// 返回 -1 要等待的子进程不存在
//      -2 要等待的子进程未结束
pub fn sys_waitpid(pid: usize, exit_code: usize) -> isize {
    syscall(SYSCALL_WAITPID, [pid, exit_code, 0])
}

pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0])
}

pub fn sys_pipe(pipe: usize) -> isize {
    syscall(SYSCALL_PIPE, [pipe, 0, 0])
}

pub fn sys_shutdown() -> isize {
    syscall(usize::MAX, [0, 0, 0])
}
