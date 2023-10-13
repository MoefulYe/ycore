use crate::mm::address::{PhysPageNum, VirtPageNum};

pub const PAGE_SIZE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 2;
pub const USER_STACK_SIZE_BY_PAGE: usize = 2;
pub const USER_STACK_SIZE: usize = PAGE_SIZE * USER_STACK_SIZE_BY_PAGE;
pub const MAX_APP_NUM: usize = 16;
pub const APP_BASE_ADDR: usize = 0x8040_0000;
pub const APP_SIZE_LIMIT: usize = 0x2_0000;
pub const CLOCK_FREQ: usize = 100000;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const MEMORY_END: usize = 0x8080_0000;
pub const PTE_SIZE: usize = 8;
pub const PTES_NUM: usize = PAGE_SIZE / PTE_SIZE;
pub const LAST_VPN: VirtPageNum = VirtPageNum(usize::MAX);
pub const TRAMPOLINE: VirtPageNum = LAST_VPN;
pub const MEM_END_PPN: PhysPageNum = PhysPageNum(MEMORY_END >> PAGE_SIZE_BITS);
pub const TRAP_CONTEXT: VirtPageNum = VirtPageNum::sub(LAST_VPN, 1);
