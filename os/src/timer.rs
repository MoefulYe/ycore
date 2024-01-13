use crate::{constant::CLOCK_FREQ, sbi::set_timer};
use log::debug;
use riscv::register::time;

const TICKS_PER_SEC: usize = 100;
const MILLIS_PER_SEC: usize = 1000;

pub fn get_time() -> usize {
    time::read()
}

pub fn set_next_trigger() {
    set_timer(get_time() + CLOCK_FREQ / TICKS_PER_SEC)
}

pub fn get_time_ms() -> usize {
    get_time() / (CLOCK_FREQ / MILLIS_PER_SEC)
}

pub fn init() {
    unsafe {
        riscv::register::sie::set_stimer();
    }
}
