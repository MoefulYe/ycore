use crate::{
    constant::{KERNEL_STACK_SIZE_BY_PAGE, TRAMPOLINE_VPN},
    mm::mem_set::KERNEL_MEM_SPACE,
    process::pid::Pid,
};

use super::address::{VirtAddr, VirtPageSpan};

//内核栈的代理对象
pub struct KernelStack;

impl KernelStack {
    // [top, bottom)
    pub fn get_postion(pid: Pid) -> VirtPageSpan {
        let offset = (KERNEL_STACK_SIZE_BY_PAGE + 1) * pid.0;
        let top = TRAMPOLINE_VPN - offset - 2usize;
        let bottom = TRAMPOLINE_VPN - offset;
        (top..bottom).into()
    }

    pub fn new(pid: Pid) -> Self {
        use crate::mm::virt_mem_area::Permission;
        let range = Self::get_postion(pid);
        KERNEL_MEM_SPACE
            .exclusive_access()
            .insert_framed_area(range, Permission::W | Permission::R);
        Self
    }

    #[allow(unused)]
    pub fn push_on_btm<T>(&self, pid: Pid, val: T) -> VirtAddr
    where
        T: Sized + 'static,
    {
        let ret = self.btm(pid) - core::mem::size_of::<T>();
        *ret.identical().as_mut() = val;
        ret
    }

    pub fn btm(&self, pid: Pid) -> VirtAddr {
        Self::get_postion(pid).end.floor()
    }
}
