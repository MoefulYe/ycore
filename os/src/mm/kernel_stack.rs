use core::ops::Range;

use crate::{
    constant::{KERNEL_STACK_SIZE_BY_PAGE, TRAMPOLINE_VPN},
    mm::mem_set::KERNEL_MEM_SPACE,
    process::pid::Pid,
};

use super::address::{VirtAddr, VirtPageNum};

//内核栈的代理对象
pub struct KernelStack;

impl KernelStack {
    // [top, bottom)
    pub fn get_postion(pid: Pid) -> Range<VirtPageNum> {
        let offset = (KERNEL_STACK_SIZE_BY_PAGE + 1) * pid.0;
        let top = TRAMPOLINE_VPN - offset - 2;
        let bottom = TRAMPOLINE_VPN - offset;
        top..bottom
    }

    pub fn new(pid: Pid) -> Self {
        use crate::mm::virt_mem_area::Permission;
        let range = Self::get_postion(pid);
        KERNEL_MEM_SPACE
            .exclusive_access()
            .insert_framed_area(range, Permission::W | Permission::R);
        Self
    }

    pub fn push_on_btm<T>(&self, pid: Pid, val: T) -> VirtAddr
    where
        T: Sized,
    {
        let ret = self.get_btm(pid) - core::mem::size_of::<T>();
        unsafe {
            *ret.raw() = val;
        }
        ret
    }

    pub fn get_btm(&self, pid: Pid) -> VirtAddr {
        Self::get_postion(pid).end.floor()
    }
}
