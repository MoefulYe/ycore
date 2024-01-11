use crate::sync::up::UPSafeCell;
use alloc::vec::Vec;
use core::{
    fmt::Display,
    ops::{Add, AddAssign, Sub, SubAssign},
};
use log::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pid(pub usize);

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
    recycle_pool: Vec<Pid>,
}

impl Allocator {
    pub fn new() -> Self {
        Self {
            current: Pid(0),
            recycle_pool: Vec::new(),
        }
    }

    pub fn alloc(&mut self) -> Pid {
        if let Some(pid) = self.recycle_pool.pop() {
            pid
        } else {
            let pid = self.current;
            self.current += 1;
            pid
        }
    }

    pub fn dealloc(&mut self, pid: Pid) {
        self.recycle_pool.push(pid);
    }
}

lazy_static! {
    pub static ref ALLOCATOR: UPSafeCell<Allocator> = unsafe { UPSafeCell::new(Allocator::new()) };
}
