use bitflags::bitflags;

use crate::{
    syscall::{sys_kill, sys_sigaction, sys_sigret, sys_sysprocmask},
    Pid, Result,
};

pub type Signal = i32;

pub const SIGDEF: i32 = 0; // Default signal handling
pub const SIGHUP: i32 = 1;
pub const SIGINT: i32 = 2;
pub const SIGQUIT: i32 = 3;
pub const SIGILL: i32 = 4;
pub const SIGTRAP: i32 = 5;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;
pub const SIGFPE: i32 = 8;
pub const SIGKILL: i32 = 9;
pub const SIGUSR1: i32 = 10;
pub const SIGSEGV: i32 = 11;
pub const SIGUSR2: i32 = 12;
pub const SIGPIPE: i32 = 13;
pub const SIGALRM: i32 = 14;
pub const SIGTERM: i32 = 15;
pub const SIGSTKFLT: i32 = 16;
pub const SIGCHLD: i32 = 17;
pub const SIGCONT: i32 = 18;
pub const SIGSTOP: i32 = 19;
pub const SIGTSTP: i32 = 20;
pub const SIGTTIN: i32 = 21;
pub const SIGTTOU: i32 = 22;
pub const SIGURG: i32 = 23;
pub const SIGXCPU: i32 = 24;
pub const SIGXFSZ: i32 = 25;
pub const SIGVTALRM: i32 = 26;
pub const SIGPROF: i32 = 27;
pub const SIGWINCH: i32 = 28;
pub const SIGIO: i32 = 29;
pub const SIGPWR: i32 = 30;
pub const SIGSYS: i32 = 31;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct SignalFlags: i32 {
        const SIGDEF = 1; // Default signal handling
        const SIGHUP = 1 << 1;
        const SIGINT = 1 << 2;
        const SIGQUIT = 1 << 3;
        const SIGILL = 1 << 4;
        const SIGTRAP = 1 << 5;
        const SIGABRT = 1 << 6;
        const SIGBUS = 1 << 7;
        const SIGFPE = 1 << 8;
        const SIGKILL = 1 << 9;
        const SIGUSR1 = 1 << 10;
        const SIGSEGV = 1 << 11;
        const SIGUSR2 = 1 << 12;
        const SIGPIPE = 1 << 13;
        const SIGALRM = 1 << 14;
        const SIGTERM = 1 << 15;
        const SIGSTKFLT = 1 << 16;
        const SIGCHLD = 1 << 17;
        const SIGCONT = 1 << 18;
        const SIGSTOP = 1 << 19;
        const SIGTSTP = 1 << 20;
        const SIGTTIN = 1 << 21;
        const SIGTTOU = 1 << 22;
        const SIGURG = 1 << 23;
        const SIGXCPU = 1 << 24;
        const SIGXFSZ = 1 << 25;
        const SIGVTALRM = 1 << 26;
        const SIGPROF = 1 << 27;
        const SIGWINCH = 1 << 28;
        const SIGIO = 1 << 29;
        const SIGPWR = 1 << 30;
        const SIGSYS = 1 << 31;
    }
}

impl Default for SignalFlags {
    fn default() -> Self {
        Self::SIGTRAP | Self::SIGQUIT
    }
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Default)]
pub struct SignalAction {
    action: usize,
    mask: SignalFlags,
}

impl SignalAction {
    pub fn new(action: fn() -> !, mask: SignalFlags) -> Self {
        Self {
            action: action as usize,
            mask,
        }
    }

    pub fn bare(mask: SignalFlags) -> Self {
        Self { action: 0, mask }
    }

    pub fn mask(&self) -> SignalFlags {
        self.mask
    }

    pub fn action(&self) -> Option<fn() -> !> {
        match self.action {
            0 => None,
            action => Some(unsafe { core::mem::transmute(action) }),
        }
    }
}

pub fn kill(pid: Pid, signal: Signal) -> Result {
    match sys_kill(pid, signal as usize) {
        -1 => Err(()),
        _ => Ok(()),
    }
}

pub fn sig_getaction(signal: Signal) -> SignalAction {
    let mut action = Default::default();
    sys_sigaction(signal as usize, 0, &mut action as *mut _ as usize);
    action
}

pub fn sig_setaction(signal: Signal, action: SignalAction) -> SignalAction {
    let mut old = Default::default();
    sys_sigaction(
        signal as usize,
        &action as *const _ as usize,
        &mut old as *mut _ as usize,
    );
    old
}

pub fn sig_procmask(mask: SignalFlags) -> Result<SignalFlags> {
    match sys_sysprocmask(mask.bits() as usize) {
        -1 => Err(()),
        old => Ok(SignalFlags::from_bits_truncate(old as i32)),
    }
}

pub fn sig_ret() -> ! {
    sys_sigret();
    unreachable!();
}
