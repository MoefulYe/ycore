use crate::constant::MEMORY_END;

use super::address::{PhysAddr, PhysPageNum};

mod symbol {
    extern "C" {
        pub fn stext();
        pub fn etext();
        pub fn srodata();
        pub fn erodata();
        pub fn sdata();
        pub fn edata();
        pub fn sbss_with_stack();
        pub fn ebss();
        pub fn ekernel();
        pub fn strampoline();
    }
}

pub fn stext() -> PhysPageNum {
    PhysAddr(symbol::stext as usize).phys_page_num()
}

pub fn etext() -> PhysPageNum {
    PhysAddr(symbol::etext as usize).phys_page_num()
}

pub fn srodata() -> PhysPageNum {
    PhysAddr(symbol::srodata as usize).phys_page_num()
}

pub fn erodata() -> PhysPageNum {
    PhysAddr(symbol::erodata as usize).phys_page_num()
}

pub fn sdata() -> PhysPageNum {
    PhysAddr(symbol::sdata as usize).phys_page_num()
}

pub fn edata() -> PhysPageNum {
    PhysAddr(symbol::edata as usize).phys_page_num()
}

pub fn sbss_with_stack() -> PhysPageNum {
    PhysAddr(symbol::sbss_with_stack as usize).phys_page_num()
}

pub fn ebss() -> PhysPageNum {
    PhysAddr(symbol::ebss as usize).phys_page_num()
}

pub fn ekernel() -> PhysPageNum {
    PhysAddr(symbol::ekernel as usize).phys_page_num()
}

pub fn strampoline() -> PhysPageNum {
    PhysAddr(symbol::strampoline as usize).phys_page_num()
}

pub const fn mem_end() -> PhysPageNum {
    PhysAddr(MEMORY_END as usize).phys_page_num()
}
