use log::info;

use crate::constant::{
    APP_BASE_ADDR, APP_SIZE_LIMIT, KERNEL_STACK_SIZE, MAX_APP_NUM, USER_STACK_SIZE,
};
use crate::trap::context::Context;
use core::arch::asm;

#[repr(align(4096))]
#[derive(Copy, Clone)]
pub struct KernelStack([u8; KERNEL_STACK_SIZE]);
impl KernelStack {
    pub fn get_sp(&self) -> usize {
        self.0.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&mut self, cx: Context) -> usize {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<Context>()) as *mut Context;
        unsafe {
            *cx_ptr = cx;
        }
        cx_ptr as usize
    }

    pub fn singleton() -> &'static mut [KernelStack; MAX_APP_NUM] {
        static mut KERNEL_STACK: [KernelStack; MAX_APP_NUM] =
            [KernelStack([0; KERNEL_STACK_SIZE]); MAX_APP_NUM];
        unsafe { &mut KERNEL_STACK }
    }
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
pub struct UserStack([u8; USER_STACK_SIZE]);
impl UserStack {
    pub fn get_sp(&self) -> usize {
        self.0.as_ptr() as usize + USER_STACK_SIZE
    }

    pub fn singleton() -> &'static mut [UserStack; MAX_APP_NUM] {
        static mut USER_STACK: [UserStack; MAX_APP_NUM] =
            [UserStack([0; USER_STACK_SIZE]); MAX_APP_NUM];
        unsafe { &mut USER_STACK }
    }
}

pub struct Loader;

extern "C" {
    fn _num_app();
}

impl Loader {
    pub fn load_apps() -> usize {
        info!("Load app binary, total {} app(s)", get_num_app());
        let num_app_ptr = _num_app as usize as *const usize;
        let num_app = get_num_app();
        let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
        unsafe {
            asm!("fence.i");
        }

        for i in 0..num_app {
            let base = get_base_i(i);
            (base..base + APP_SIZE_LIMIT)
                .for_each(|addr| unsafe { (addr as *mut u8).write_volatile(0) });
            let src = unsafe {
                core::slice::from_raw_parts(
                    app_start[i] as *const u8,
                    app_start[i + 1] - app_start[i],
                )
            };
            let dst = unsafe { core::slice::from_raw_parts_mut(base as *mut u8, src.len()) };
            dst.copy_from_slice(src);
        }
        num_app
    }
}

pub fn get_num_app() -> usize {
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

pub fn get_base_i(app_id: usize) -> usize {
    APP_BASE_ADDR + app_id * APP_SIZE_LIMIT
}
