use crate::{
    constant::{PAGE_MASK, PAGE_SIZE, TRAP_CONTEXT_VPN},
    mm::{
        address::{PhysPageNum, VirtAddr},
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
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    //task上下文
    pub context: Context,
    //进程状态
    pub state: State,
    //内存描述符
    pub mem_set: MemSet,
    //trap上下文的物理页号
    pub trap_ctx_ppn: PhysPageNum,
    //记录消耗了多少内存
    pub base_size: usize,
    //堆底
    pub heap_bottm: usize,
    //堆顶
    pub brk: usize,
}

impl TaskControlBlock {
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (mem_set, user_sp, entry) = MemSet::from_elf(elf_data);
        let trap_ctx_ppn = mem_set.translate(TRAP_CONTEXT_VPN).unwrap().ppn();
        let state = State::Ready;
        let kernel_stack_range = kernel_stack::get_postion(app_id);
        KERNEL_MEM_SPACE
            .exclusive_access()
            .insert_framed_area(kernel_stack_range.clone(), Permission::W | Permission::R);
        let tcb = Self {
            context: Context::goto_trap_return(kernel_stack_range.end.floor().0),
            state,
            mem_set,
            trap_ctx_ppn,
            base_size: user_sp.floor().0,
            heap_bottm: user_sp.floor().0,
            brk: user_sp.floor().0,
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

    pub fn recycle(&mut self) {
        self.mem_set.recycle();
    }

    //改变堆顶, 成功时返回旧的堆顶, 失败时返回usize::MAX
    pub fn change_prk(&mut self, size: isize) -> usize {
        //如果申请的内存不是页对齐的, 则返回错误
        if size as usize & PAGE_MASK != 0 {
            return usize::MAX;
        }
        let old = self.brk;
        let new = (self.brk as isize + size) as usize;
        //如果堆顶超过了堆底, 则返回错误
        if new < self.heap_bottm {
            return usize::MAX;
        }
        let old_ppn = VirtAddr(old).floor();
        let new_ppn = VirtAddr(new).floor();
        if old_ppn == new_ppn {
            return old;
        } else if old_ppn < new_ppn {
            self.mem_set.heap_grow(new_ppn);
        } else {
            self.mem_set.heap_shrink(new_ppn);
        }
        self.brk = new;
        return old;
    }
}
