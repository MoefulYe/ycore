#[derive(Copy, Clone)]
#[repr(C)]
pub struct Context {
    //从switch返回后sp指向的位置
    pub ra: usize,
    //栈顶指针
    pub sp: usize,
    //Callee saved
    pub s_regs: [usize; 12],
}

impl Default for Context {
    fn default() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s_regs: [0; 12],
        }
    }
}

impl Context {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn goto_restore(sp: usize) -> Self {
        extern "C" {
            fn __restore(cx_addr: usize);
        }
        Self {
            ra: __restore as usize,
            sp,
            ..Default::default()
        }
    }
}
