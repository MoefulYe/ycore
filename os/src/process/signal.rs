#![allow(unused)]
use bitflags::bitflags;
use log::info;

use crate::process::processor::PROCESSOR;

use super::pcb::ProcessControlBlock;

pub type Signal = i32;
pub const SIGDEF: Signal = 0; // Default signal handling
pub const SIGHUP: Signal = 1;
pub const SIGINT: Signal = 2;
pub const SIGQUIT: Signal = 3;
pub const SIGILL: Signal = 4;
pub const SIGTRAP: Signal = 5;
pub const SIGABRT: Signal = 6;
pub const SIGBUS: Signal = 7;
pub const SIGFPE: Signal = 8;
pub const SIGKILL: Signal = 9;
pub const SIGUSR1: Signal = 10;
pub const SIGSEGV: Signal = 11;
pub const SIGUSR2: Signal = 12;
pub const SIGPIPE: Signal = 13;
pub const SIGALRM: Signal = 14;
pub const SIGTERM: Signal = 15;
pub const SIGSTKFLT: Signal = 16;
pub const SIGCHLD: Signal = 17;
pub const SIGCONT: Signal = 18;
pub const SIGSTOP: Signal = 19;
pub const SIGTSTP: Signal = 20;
pub const SIGTTIN: Signal = 21;
pub const SIGTTOU: Signal = 22;
pub const SIGURG: Signal = 23;
pub const SIGXCPU: Signal = 24;
pub const SIGXFSZ: Signal = 25;
pub const SIGVTALRM: Signal = 26;
pub const SIGPROF: Signal = 27;
pub const SIGWINCH: Signal = 28;
pub const SIGIO: Signal = 29;
pub const SIGPWR: Signal = 30;
pub const SIGSYS: Signal = 31;

pub const MAX_SIG: usize = 31;

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

        const HANDLE_BY_KERNEL = SIGKILL | SIGSTOP | SIGCONT | SIGDEF;
    }
}

impl Default for SignalFlags {
    fn default() -> Self {
        Self::SIGTRAP | Self::SIGQUIT
    }
}

impl SignalFlags {
    pub fn code(self) -> usize {
        self.bits().trailing_zeros() as usize
    }

    pub fn check_error(&self) -> Option<(i32, &'static str)> {
        if self.contains(Self::SIGINT) {
            Some((-2, "SIGINT"))
        } else if self.contains(Self::SIGILL) {
            Some((-4, "SIGILL"))
        } else if self.contains(Self::SIGABRT) {
            Some((-6, "SIGABRT"))
        } else if self.contains(Self::SIGFPE) {
            Some((-8, "SIGFPE"))
        } else if self.contains(Self::SIGKILL) {
            Some((-9, "SIGKILL"))
        } else if self.contains(Self::SIGSEGV) {
            Some((-11, "SIGSEGV"))
        } else {
            None
        }
    }
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}

pub type SignalActions = [SignalAction; MAX_SIG + 1];
