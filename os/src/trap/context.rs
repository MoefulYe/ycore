use riscv::register::sstatus::{self, Sstatus, SPP};
/// Trap Context
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Context {
    /// general regs[0..31]
    pub x: [usize; 32],
    /// CSR sstatus      
    pub sstatus: Sstatus,
    /// CSR sepc
    pub sepc: usize,
    // 内核空间中该用户态程序的页表地址
    pub kernel_satp: usize,
    // 内核空间中该用户态程序的内核栈地址
    pub kernel_sp: usize,
    // 内核空间中trap_handler的入口地址
    pub trap_handler: usize,
}

impl Context {
    /// set stack pointer to x_2 reg (sp)
    pub fn set_sp(&mut self, sp: usize) {
        self.x[2] = sp;
    }
    /// init app context
    pub fn new(
        entry: usize,
        sp: usize,
        kernel_satp: usize,
        kernel_sp: usize,
        trap_handler: usize,
    ) -> Self {
        let mut sstatus = sstatus::read(); // CSR sstatus
        sstatus.set_spp(SPP::User); //previous privilege mode: user mode
        let mut cx = Self {
            x: [0; 32],
            sstatus,
            sepc: entry,  // entry point of app
            kernel_satp,  // addr of page table
            kernel_sp,    // kernel stack
            trap_handler, // addr of trap_handler function
        };
        cx.set_sp(sp); // app's user stack pointer
        cx // return initial Trap Context of app
    }
}
