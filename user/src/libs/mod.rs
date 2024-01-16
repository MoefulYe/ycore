#[macro_use]
pub mod console;
pub mod io;
pub mod signal;
pub mod types;
use crate::syscall::{
    sys_exec, sys_exit, sys_fork, sys_getpid, sys_gettime, sys_sbrk, sys_shutdown, sys_waitpid,
    sys_yield,
};

pub use self::console::*;
pub use self::io::*;
pub use self::signal::*;
pub use self::types::*;

pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code as usize);
    panic!("unreachable after sys_exit!");
}

pub fn yield_() {
    sys_yield();
}

pub fn time() -> Ms {
    sys_gettime() as Ms
}

pub fn sbrk(size: isize) -> Result<isize> {
    let ret = sys_sbrk(size as usize);
    if ret < 0 {
        Err(())
    } else {
        Ok(ret)
    }
}

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

pub fn exec(path: &str, args: &[CStr]) -> ! {
    sys_exec(path.as_ptr() as usize, args.as_ptr() as usize);
    panic!("unreachable after sys_exec!");
}

pub fn waitpid(pid: Pid) -> Result<(Pid, ExitCode), ()> {
    let mut exit_code: i32 = 0;
    loop {
        match sys_waitpid(pid, &mut exit_code as *mut _ as usize) {
            -1 => break Err(()),
            -2 => yield_(),
            exit_pid => break Ok((exit_pid as Pid, exit_code)),
        }
    }
}

pub fn wait() -> (Pid, ExitCode) {
    const ANY: isize = -1;
    let mut exit_code: i32 = 0;
    loop {
        match sys_waitpid(ANY as usize, &mut exit_code as *mut _ as usize) {
            -2 => yield_(),
            exit_pid => break (exit_pid as Pid, exit_code),
        }
    }
}

pub fn getpid() -> Pid {
    sys_getpid() as Pid
}

pub fn shutdown() -> ! {
    sys_shutdown();
    unreachable!()
}

pub fn sleep(duration: Ms) {
    let start = time();
    while time() < start + duration {
        yield_();
    }
}
