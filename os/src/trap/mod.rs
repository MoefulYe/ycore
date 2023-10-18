#![allow(unused)]
use crate::{
    constant::{TRAMPOLINE_VA, TRAP_CONTEXT_VA},
    mm::address::VirtAddr,
    sbi::shutdown,
    syscall::syscall,
    task::SCHEDULER,
};

use self::context::Context;
use core::arch::{asm, global_asm};
use log::{error, info};
use riscv::register::{mtvec::TrapMode, scause, stval, stvec};

pub mod context;

global_asm!(include_str!("trap.asm"));

pub unsafe fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE_VA.0, TrapMode::Direct);
    }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    error!("[kernel] a trap from kernel");
    shutdown(false);
}

#[no_mangle]
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let cx = SCHEDULER.exclusive_access().get_current_trap_ctx();
    let scause = scause::read();
    let stval = stval::read();
    use scause::Exception::*;
    use scause::Interrupt::*;
    use scause::Trap::*;
    match scause.cause() {
        Interrupt(i) => match i {
            SupervisorTimer => {
                info!("[timer] timeslice used up, switch process!");
                SCHEDULER.exclusive_access().suspend_current().schedule();
            }
            _ => panic!(
                "[trap-handler] unsupported interrupt: {:?}, scause: {:#x}, stval: {:#x}",
                scause.cause(),
                scause.bits(),
                stval
            ),
        },
        Exception(e) => match e {
            UserEnvCall => {
                let id = cx.x[17];
                let args = [cx.x[10], cx.x[11], cx.x[12]];
                cx.sepc += 4;
                cx.x[10] = syscall(id, args) as usize;
            }
            IllegalInstruction => {
                error!(
                    "[trap-handler] IllegalInstruction at {:#x}, bad instruction {:#x?} This proccess will be killed!",
                    cx.sepc, stval
                );
                SCHEDULER.exclusive_access().recycle_current().schedule();
            }
            StorePageFault | StoreFault | LoadFault | LoadPageFault => {
                error!("[trap-handler] PageFault in application, the proccess will be killed");
                SCHEDULER.exclusive_access().recycle_current().schedule();
            }
            _ => panic!(
                "[trap-handler] unsupported exception: {:?}, scause: {:#x}, stval: {:#x}",
                scause.cause(),
                scause.bits(),
                stval
            ),
        },
    }
    trap_return()
}

#[no_mangle]
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let VirtAddr(trap_cx_ptr) = TRAP_CONTEXT_VA;
    let user_satp = SCHEDULER.exclusive_access().get_current_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let VirtAddr(restore_va) = TRAMPOLINE_VA + (__restore as usize - __alltraps as usize);
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,
            in("a1") user_satp,
            options(noreturn)
        );
    }
}
