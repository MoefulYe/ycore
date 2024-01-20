#![allow(unused)]
use crate::mm::address::{PhysPageNum, VirtAddr, VirtPageNum};

pub const MAX_APP_NUM: usize = 16;
pub const APP_BASE_ADDR: usize = 0x8040_0000;
pub const APP_SIZE_LIMIT: usize = 0x2_0000;
pub const CLOCK_FREQ: usize = 1250_0000;
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const MEMORY_END: usize = 0x8100_0000;

pub const PTE_SIZE: usize = 8;
pub const PTES_NUM: usize = PAGE_SIZE / PTE_SIZE;

pub const PAGE_SIZE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;
pub const PAGE_MASK: usize = PAGE_SIZE - 1;

pub const PA_WIDTH: usize = 56;
pub const PA_MASK: usize = (1 << PA_WIDTH) - 1;
pub const VA_WIDTH: usize = 39;
pub const VA_MASK: usize = (1 << VA_WIDTH) - 1;
pub const PPN_WIDTH: usize = PA_WIDTH - PAGE_SIZE_BITS;
pub const PPN_MASK: usize = (1 << PPN_WIDTH) - 1;
pub const VPN_WIDTH: usize = VA_WIDTH - PAGE_SIZE_BITS;
pub const VPN_MASK: usize = (1 << VPN_WIDTH) - 1;

pub const LAST_VPN: VirtPageNum = VirtPageNum(usize::MAX);
pub const TRAMPOLINE_VPN: VirtPageNum = LAST_VPN;
pub const TRAMPOLINE_VA: VirtAddr = VirtAddr(TRAMPOLINE_VPN.0 << PAGE_SIZE_BITS);
pub const MEM_END_PPN: PhysPageNum = PhysPageNum(MEMORY_END >> PAGE_SIZE_BITS);
pub const TRAP_CONTEXT_VPN: VirtPageNum = VirtPageNum(LAST_VPN.0 - 1);
pub const TRAP_CONTEXT_VA: VirtAddr = VirtAddr(TRAP_CONTEXT_VPN.0 << PAGE_SIZE_BITS);

pub const KERNEL_STACK_SIZE_BY_PAGE: usize = 2;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * KERNEL_STACK_SIZE_BY_PAGE;
pub const USER_STACK_SIZE_BY_PAGE: usize = 2;
pub const USER_STACK_SIZE: usize = PAGE_SIZE * USER_STACK_SIZE_BY_PAGE;

pub const VIRTIO0: (usize, usize) = (0x1000_1000, 0x1000);
pub const MMIO: &[(usize, usize)] = &[VIRTIO0];

pub mod exit_code {
    pub const SUCCESS: i32 = 0;
    pub const ILLEGAL_INSTRUCTION: i32 = -1;
    pub const LOAD_STORE_FAULT: i32 = -2;
}
