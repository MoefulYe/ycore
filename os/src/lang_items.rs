use log::error;

use crate::{fs::inode::YFS, sbi::shutdown};
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    YFS.flush();
    if let Some(location) = info.location() {
        error!(
            "[kernel] Panicked at {}:{} {}. shutting down...",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        error!(
            "[kernel] Panicked: {}. shutting down...",
            info.message().unwrap()
        );
    }
    shutdown(true)
}
