use core::fmt::{self, Write};

use super::{fread, fwrite};

struct Stdout;
struct Stderr;

pub const STDIN: usize = 0;
pub const STDOUT: usize = 1;
#[allow(dead_code)]
pub const STDERR: usize = 2;

pub fn getchar() -> u8 {
    let mut c = [0u8; 1];
    fread(STDIN, &mut c).unwrap();
    c[0]
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        fwrite(STDOUT, s.as_bytes()).unwrap();
        Ok(())
    }
}

impl Write for Stderr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        fwrite(STDERR, s.as_bytes()).unwrap();
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
}

pub fn eprint(args: fmt::Arguments) {
    Stderr.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! eprint {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::eprint(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! eprintln {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::eprint(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}
