use crate::{syscall::syscall, task::Scheduler};

use self::context::Context;
use core::arch::global_asm;
use log::{debug, error, info};
use riscv::register::{mtvec::TrapMode, scause, stval, stvec};

pub mod context;

global_asm!(include_str!("trap.asm"));

pub unsafe fn init() {
    extern "C" {
        fn __alltraps();
    }
    stvec::write(__alltraps as usize, TrapMode::Direct);
    info!("[kernel] set trap_handler! ");
}

#[no_mangle]
pub fn trap_handler(cx: &mut Context) -> &mut Context {
    let scause = scause::read();
    let stval = stval::read();
    use scause::Exception::*;
    // use scause::Interrupt::*;
    use scause::Trap::*;
    debug!(
        "[kernel] Trap: {:?}, scause: {:#x}, stval: {:#x}",
        scause.cause(),
        scause.bits(),
        stval
    );
    match scause.cause() {
        Interrupt(i) => match i {
            _ => panic!("[kernel] unsupported"),
        },
        Exception(e) => match e {
            IllegalInstruction => {
                error!(
                    "[kernel] IllegalInstruction at {:#x}, bad instruction {:#x?}\nThis proccess will be killed!",
                    cx.sepc, stval
                );
                todo!();
                Scheduler::singletion().run_next()
            }
            StorePageFault | StoreFault => {
                error!("[kernel] PageFault in application, the proccess will be killed");
                todo!();
                Scheduler::singletion().run_next()
            }
            UserEnvCall => {
                let id = cx.x[17];
                let args = [cx.x[10], cx.x[11], cx.x[12]];
                cx.sepc += 4;
                cx.x[10] = syscall(id, args) as usize;
            }
            _ => panic!("[kernel] unsupported"),
        },
    }
    cx
}
