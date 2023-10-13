use crate::{
    constant::TRAP_CONTEXT,
    mm::{
        address::PhysPageNum,
        kernel_stack,
        mem_set::{MemSet, KERNEL_MEM_SPACE},
        virt_mem_area::Permission,
    },
    trap::context::Context as TrapContext,
    trap::trap_handler,
};

use super::context::Context;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Uninit,
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub context: Context,
    pub state: State,
    pub mem_set: MemSet,
    pub trap_ctx_ppn: PhysPageNum,
    pub base_size: usize,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (mem_set, user_sp, entry) = MemSet::from_elf(elf_data);
        let trap_cx_ppn = mem_set.translate(TRAP_CONTEXT).unwrap().ppn();
        let state = State::Ready;
        let kernel_stack_range = kernel_stack::get_postion(app_id);
        KERNEL_MEM_SPACE
            .exclusive_access()
            .insert_framed_area(kernel_stack_range, Permission::W | Permission::R);
        let tcb = Self {
            context: super::context::Context::goto_restore(kernel_stack_range.end.floor().0),
            state,
            mem_set,
            trap_ctx_ppn: trap_cx_ppn,
            base_size: user_sp.floor().0,
        };
        let trap_ctx = tcb.get_trap_ctx();
        *trap_ctx = TrapContext::new(
            entry.0,
            user_sp.floor().0,
            KERNEL_MEM_SPACE.exclusive_access().token(),
            kernel_stack_range.end.floor().0,
            trap_handler as usize,
        );
        tcb
    }

    pub fn get_trap_ctx(&self) -> &'static mut TrapContext {
        self.trap_ctx_ppn.read_as()
    }
}
