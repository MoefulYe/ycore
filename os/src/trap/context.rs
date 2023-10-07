use riscv::register::sstatus::{self, SPP};

#[repr(C)]
pub struct Context {
    pub x: [usize; 32], // General registers
    pub sstatus: usize, // Supervisor Status Register
    pub sepc: usize,    // Supervisor exception program counter
}

impl Context {
    pub fn set_sp(&mut self, value: usize) {
        self.x[2] = value;
    }

    pub fn new(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(SPP::User);
        let mut cx = Context {
            x: [0; 32],
            sstatus: sstatus.bits(),
            sepc: entry,
        };
        cx.set_sp(sp);
        cx
    }
}
