use core::ops::Range;

use crate::constant::{KERNEL_STACK_SIZE_BY_PAGE, TRAMPOLINE};

use super::address::VirtPageNum;

// [top, bottom)
pub fn get_postion(app_id: usize) -> Range<VirtPageNum> {
    let offset = (KERNEL_STACK_SIZE_BY_PAGE + 1) * app_id;
    let top = TRAMPOLINE - offset - 2;
    let bottom = TRAMPOLINE - offset;
    top..bottom
}
