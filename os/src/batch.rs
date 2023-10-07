use crate::trap::context::Context;
use crate::{sbi::shutdown, trap};
use core::arch::asm;
use log::{info, warn};

const PAGE_SIZE: usize = 4096;
const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 2;
const USER_STACK_SIZE: usize = PAGE_SIZE * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDR: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x200000;

#[repr(align(4096))]
struct KernelStack([u8; KERNEL_STACK_SIZE]);
impl KernelStack {
    pub fn get_sp(&self) -> usize {
        self.0.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    pub fn push_context(&mut self, cx: Context) -> *const Context {
        let cx_ptr = (self.get_sp() - core::mem::size_of::<Context>()) as *mut Context;
        unsafe {
            cx_ptr.write_volatile(cx);
        }
        cx_ptr
    }

    pub fn singleton() -> &'static mut KernelStack {
        unsafe { &mut KERNEL_STACK }
    }
}

static mut KERNEL_STACK: KernelStack = KernelStack([0; KERNEL_STACK_SIZE]);

#[repr(align(4096))]
struct UserStack([u8; USER_STACK_SIZE]);
impl UserStack {
    pub fn get_sp(&self) -> usize {
        self.0.as_ptr() as usize + USER_STACK_SIZE
    }

    pub fn singleton() -> &'static mut UserStack {
        unsafe { &mut USER_STACK }
    }
}
static mut USER_STACK: UserStack = UserStack([0; USER_STACK_SIZE]);

pub struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

static mut APP_MANAGER: AppManager = AppManager {
    num_app: 0,
    current_app: 0,
    app_start: [0; MAX_APP_NUM + 1],
};

impl AppManager {
    pub fn print_app_info(&self) {
        info!("[kernel] num_app: {}", self.num_app);
        for i in 0..self.num_app {
            info!(
                "[kernel] app_start[{}]: [{:#x}, {:#x})",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    pub fn current(&self) -> usize {
        self.current_app
    }

    pub fn next(&mut self) {
        self.current_app += 1;
    }

    pub fn load(&mut self) -> &mut Self {
        unsafe {
            if self.current_app >= self.num_app {
                warn!("[kernel] No more app to load!");
                shutdown(false);
            }

            (APP_BASE_ADDR..APP_BASE_ADDR + APP_SIZE_LIMIT).for_each(|a| {
                (a as *mut u8).write_volatile(0);
            });

            let app_src = core::slice::from_raw_parts(
                self.app_start[self.current_app] as *const u8,
                self.app_start[self.current_app + 1] - self.app_start[self.current_app],
            );
            let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDR as *mut u8, app_src.len());
            app_dst.copy_from_slice(app_src);

            asm!("fence.i");
            info!(
                "[kernel] Load app {}, total {}",
                self.current_app, self.num_app
            );
            self
        }
    }

    pub fn load_next(&mut self) -> &mut Self {
        self.next();
        self.load()
    }

    pub fn run_app(&self) {
        info!(
            "[kernel] Run app{}, total {}",
            self.current_app, self.num_app
        );
        extern "C" {
            fn __restore(cx_addr: usize);
        }

        unsafe {
            let init_context =
                trap::context::Context::new(APP_BASE_ADDR, UserStack::singleton().get_sp());
            let cx_ptr = KernelStack::singleton().push_context(init_context) as usize;
            __restore(cx_ptr)
        }
    }

    pub fn singleton() -> &'static mut AppManager {
        unsafe { &mut APP_MANAGER }
    }

    pub unsafe fn init() {
        let mut manager = Self::singleton();
        extern "C" {
            fn _num_app();
        }
        let num_app_ptr = _num_app as usize as *const usize;
        let num_app = num_app_ptr.read_volatile();
        let app_start_raw = core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);

        manager.current_app = 0;
        manager.num_app = num_app;
        manager.app_start[..=num_app].copy_from_slice(app_start_raw);
    }
}
