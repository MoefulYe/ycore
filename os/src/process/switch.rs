use super::context::Context;
use core::arch::global_asm;

global_asm!(include_str!("switch.asm"));

extern "C" {
    /// Switch to the context of `next_task_cx_ptr`, saving the current context
    /// in `current_task_cx_ptr`.
    pub fn __switch(current_task_cx_ptr: *mut Context, next_task_cx_ptr: *const Context);
}
