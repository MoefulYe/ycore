pub const PAGE_SIZE: usize = 4096;
pub const KERNEL_STACK_SIZE: usize = PAGE_SIZE * 2;
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2;
pub const MAX_APP_NUM: usize = 16;
pub const APP_BASE_ADDR: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;
