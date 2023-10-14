use core::ops::Range;

use crate::constant::{KERNEL_STACK_SIZE_BY_PAGE, TRAMPOLINE_VPN};

use super::address::VirtPageNum;

// [top, bottom)
pub fn get_postion(app_id: usize) -> Range<VirtPageNum> {
    let offset = (KERNEL_STACK_SIZE_BY_PAGE + 1) * app_id;
    let top = TRAMPOLINE_VPN - offset - 2;
    let bottom = TRAMPOLINE_VPN - offset;
    top..bottom
}
