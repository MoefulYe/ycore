use crate::mm::address::{PhysPageNum, VirtAddr, VirtPageNum};

pub const MAX_APP_NUM: usize = 16;
pub const APP_BASE_ADDR: usize = 0x8040_0000;
pub const APP_SIZE_LIMIT: usize = 0x2_0000;
pub const CLOCK_FREQ: usize = 100000;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const MEMORY_END: usize = 0x8080_0000;

pub const PTE_SIZE: usize = 8;
pub const PTES_NUM: usize = PAGE_SIZE / PTE_SIZE;

pub const PAGE_SIZE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;

pub const PA_WIDTH: usize = 56;
pub const VA_WIDTH: usize = 39;
pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_SIZE_BITS;
pub const VPN_WIDTH: usize = VA_WIDTH - PAGE_SIZE_BITS;

pub const LAST_VPN: VirtPageNum = VirtPageNum(usize::MAX);
pub const TRAMPOLINE_VPN: VirtPageNum = LAST_VPN;
pub const TRAMPOLINE_VA: VirtAddr = VirtAddr(TRAMPOLINE_VPN.0 << PAGE_SIZE_BITS);
pub const MEM_END_PPN: PhysPageNum = PhysPageNum(MEMORY_END >> PAGE_SIZE_BITS);
pub const TRAP_CONTEXT_VPN: VirtPageNum = VirtPageNum::sub(LAST_VPN, 1);
pub const TRAP_CONTEXT_VA: VirtAddr = VirtAddr(TRAP_CONTEXT_VPN.0 << PAGE_SIZE_BITS);

pub const KERNEL_STACK_SIZE_BY_PAGE: usize = 2;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * KERNEL_STACK_SIZE_BY_PAGE;
pub const USER_STACK_SIZE_BY_PAGE: usize = 2;
pub const USER_STACK_SIZE: usize = PAGE_SIZE * USER_STACK_SIZE_BY_PAGE;
