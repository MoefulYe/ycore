use core::iter::once;
use core::mem::size_of;

use crate::fs::stdio::{stderr, stdin, stdout};
use crate::mm::page_table::TopLevelEntry;
use crate::types::CStr;
use crate::{
    constant::{PAGE_MASK, TRAP_CONTEXT_VPN},
    fs::File,
    mm::{
        address::{PhysPageNum, VirtAddr},
        kernel_stack::KernelStack,
        mem_set::{MemSet, KERNEL_MEM_SPACE},
    },
    process::{context::Context as TaskContext, pid::Pid},
    trap::context::Context as TrapContext,
    trap::trap_handler,
};
use alloc::string::String;
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};

use super::pid;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ready,
    Running,
    Zombie,
}

type FdTable = Vec<Option<Arc<dyn File + Send + Sync>>>;

pub struct ProcessControlBlock {
    // 在整个生命周期中, pid不会改变
    pub pid: Pid,
    // 内核栈的代理对象, 在整个生命周期中, 该对象代理的内核栈不会改变
    pub kernel_stack: KernelStack,
    //task上下文
    pub task_context: TaskContext,
    //进程状态
    pub state: State,
    //内存描述符
    pub mem_set: MemSet,
    //trap上下文的物理页号
    pub trap_ctx_ppn: PhysPageNum,
    //记录消耗了多少内存
    pub base_size: usize,
    //堆底
    pub heap_btm: usize,
    //堆顶
    pub brk: usize,
    pub exit_code: i32,
    pub children: Vec<*mut Self>,
    //nullable
    pub parent: *mut Self,
    pub fd_table: FdTable,
}

impl Drop for ProcessControlBlock {
    fn drop(&mut self) {
        pid::ALLOCATOR.exclusive_access().dealloc(self.pid);
        // drop(self.children);
        // drop(self.mem_set);
    }
}

unsafe impl Send for ProcessControlBlock {}

impl ProcessControlBlock {
    pub fn task_ctx(&mut self) -> *mut TaskContext {
        &mut self.task_context as *mut _
    }

    pub fn initproc(elf_data: &[u8]) -> Self {
        let (mem_set, user_sp, entry) = MemSet::from_elf(elf_data);

        //得到中断上下文的物理页号
        let trap_ctx_ppn = mem_set.translate(TRAP_CONTEXT_VPN).unwrap().ppn();
        let pid = pid::ALLOCATOR.exclusive_access().alloc();
        let kernel_stack = KernelStack::new(pid);

        let user_stack_btm = user_sp.floor().0;
        let kernel_stack_btm = kernel_stack.btm(pid).0;

        let pcb = Self {
            pid,
            state: State::Ready,
            kernel_stack,
            mem_set,
            trap_ctx_ppn,
            task_context: TaskContext::goto_trap_return(kernel_stack_btm),
            base_size: user_stack_btm,
            heap_btm: user_stack_btm,
            brk: user_stack_btm,
            exit_code: 0,
            children: vec![],
            parent: core::ptr::null_mut(),
            fd_table: vec![Some(stdin()), Some(stdout()), Some(stderr())],
        };
        *pcb.trap_ctx() = TrapContext::new(
            entry.0,
            user_stack_btm,
            KERNEL_MEM_SPACE.exclusive_access().token(),
            kernel_stack_btm,
            trap_handler as usize,
        );
        pcb
    }

    pub fn fork(&mut self) -> *mut Self {
        let mem_set = self.mem_set.clone();
        let trap_ctx_ppn = mem_set.translate(TRAP_CONTEXT_VPN).unwrap().ppn();
        let pid = pid::ALLOCATOR.exclusive_access().alloc();
        let kernel_stack = KernelStack::new(pid);

        let kernel_stack_btm = kernel_stack.btm(pid).0;
        let ret = Box::leak(Box::new(ProcessControlBlock {
            pid,
            kernel_stack,
            task_context: TaskContext::goto_trap_return(kernel_stack_btm),
            state: State::Ready,
            mem_set,
            trap_ctx_ppn,
            base_size: self.base_size,
            heap_btm: self.heap_btm,
            brk: self.brk,
            exit_code: 0,
            children: Vec::new(),
            parent: self as *mut Self,
            fd_table: self.fd_table.clone(),
        })) as *mut Self;
        unsafe {
            let ret = &mut *ret;
            ret.trap_ctx().kernel_sp = kernel_stack_btm;
        }
        self.children.push(ret);
        ret
    }

    pub fn exec(&mut self, elf_data: &[u8], argv: Vec<String>) {
        let (mem_set, user_sp, entry) = MemSet::from_elf(elf_data);
        //得到中断上下文的物理页号
        let trap_ctx_ppn = mem_set.translate(TRAP_CONTEXT_VPN).unwrap().ppn();

        self.mem_set = mem_set;
        self.trap_ctx_ppn = trap_ctx_ppn;

        let user_stack_btm = user_sp.floor().0;
        self.base_size = user_stack_btm;
        self.heap_btm = user_stack_btm;
        self.brk = user_stack_btm;
        let argc = argv.len();
        let argv_base = user_stack_btm - size_of::<CStr>() * (argc + 1);
        let page_table = self.page_table();

        // argv[argv.len()] = null
        *page_table.translate_virt_mut((user_stack_btm - size_of::<CStr>()) as *mut CStr) =
            core::ptr::null();

        let mut base = argv_base;
        for (i, arg) in argv.into_iter().enumerate() {
            let ptr = argv_base + size_of::<CStr>() * i;
            base = base - arg.len() - 1;
            *page_table.translate_virt_mut(ptr as *mut CStr) = base as CStr;
            for (j, c) in arg.bytes().chain(once(b'\0')).enumerate() {
                *page_table.translate_virt_mut((base + j) as *mut u8) = c;
            }
        }

        let kernel_stack_btm = self.kernel_stack.btm(self.pid).0;
        *self.trap_ctx() = TrapContext::new(
            entry.0,
            base,
            KERNEL_MEM_SPACE.exclusive_access().token(),
            kernel_stack_btm,
            trap_handler as usize,
        );
        let regs = &mut self.trap_ctx().x;
        regs[10] = argc;
        regs[11] = argv_base;
    }

    pub fn token(&self) -> usize {
        self.mem_set.token()
    }

    pub fn page_table(&self) -> TopLevelEntry {
        TopLevelEntry::from_token(self.token())
    }

    pub fn trap_ctx(&self) -> &'static mut TrapContext {
        self.trap_ctx_ppn.read_as()
    }

    pub fn pid(&self) -> Pid {
        self.pid
    }

    pub fn is_zombie(&self) -> bool {
        self.state == State::Zombie
    }

    pub fn recycle(&mut self) {
        self.mem_set.recycle();
    }

    //改变堆顶, 成功时返回旧的堆顶, 失败时返回usize::MAX
    pub fn change_brk(&mut self, size: isize) -> usize {
        //如果申请的内存不是页对齐的, 则返回错误
        if size as usize & PAGE_MASK != 0 {
            return usize::MAX;
        }
        let old = self.brk;
        let new = (self.brk as isize + size) as usize;
        //如果堆顶超过了堆底, 则返回错误
        if new < self.heap_btm {
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

    // 添加fd表项
    pub fn add_fd(&mut self, file: Arc<dyn File + Send + Sync>) -> usize {
        if let Some((idx, entry)) = self
            .fd_table
            .iter_mut()
            .enumerate()
            .find(|(_, entry)| entry.is_none())
        {
            *entry = Some(file);
            idx
        } else {
            self.fd_table.push(Some(file));
            self.fd_table.len()
        }
    }

    pub fn close_fd(&mut self, fd: usize) -> isize {
        if let Some(entry) = self.fd_table.get_mut(fd) {
            if entry.is_none() {
                -1
            } else {
                *entry = None;
                0
            }
        } else {
            -1
        }
    }

    pub fn fd_at(&mut self, fd: usize) -> Option<Arc<dyn File + Send + Sync>> {
        if let Some(Some(entry)) = self.fd_table.get(fd) {
            Some(entry.clone())
        } else {
            None
        }
    }
}
