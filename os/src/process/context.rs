use crate::trap::trap_return;

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
    pub fn idle() -> Self {
        Default::default()
    }

    pub fn goto_trap_return(sp: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp,
            ..Default::default()
        }
    }
}
