use crate::sync::up::UPSafeCell;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};

use super::pcb::ProcessControlBlock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pid(pub usize);

impl From<usize> for Pid {
    fn from(pid: usize) -> Self {
        Pid(pid)
    }
}

impl Display for Pid {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Pid {
    pub const ANY: Pid = Pid((-1 as isize) as usize);
}

impl Add<usize> for Pid {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Pid(self.0 + rhs)
    }
}

impl AddAssign<usize> for Pid {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub<usize> for Pid {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Pid(self.0 - rhs)
    }
}

impl SubAssign<usize> for Pid {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl Sub for Pid {
    type Output = isize;

    fn sub(self, rhs: Self) -> Self::Output {
        (self.0 as isize) - (rhs.0 as isize)
    }
}

pub struct Allocator {
    current: Pid,
    pool: Vec<Pid>,
}

impl Allocator {
    pub fn new() -> Self {
        Self {
            current: Pid(0),
            pool: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Pid {
        if let Some(pid) = self.pool.pop() {
            pid
        } else {
            let pid = self.current;
            self.current += 1;
            pid
        }
    }

    pub fn dealloc(&mut self, pid: Pid) {
        self.pool.push(pid);
    }
}

lazy_static! {
    pub static ref ALLOCATOR: UPSafeCell<Allocator> = unsafe { UPSafeCell::new(Allocator::new()) };
    pub static ref PID2TASK: UPSafeCell<BTreeMap<Pid, *mut ProcessControlBlock>> =
        unsafe { UPSafeCell::new(BTreeMap::new()) };
}

pub fn task_find(pid: impl Into<Pid>) -> Option<*mut ProcessControlBlock> {
    PID2TASK.exclusive_access().get(&pid.into()).map(|ok| *ok)
}

pub fn task_insert(pid: impl Into<Pid>, task: *mut ProcessControlBlock) {
    PID2TASK.exclusive_access().insert(pid.into(), task);
}

pub fn task_delete(pid: impl Into<Pid>) {
    PID2TASK.exclusive_access().remove(&pid.into());
}
