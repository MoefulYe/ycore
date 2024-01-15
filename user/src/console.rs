use core::fmt::{self, Write};

struct Stdout;

const STDIN: usize = 0;
const STDOUT: usize = 1;
#[allow(dead_code)]
const STDERR: usize = 2;

pub fn getchar() -> u8 {
    let mut c = [0u8; 1];
    crate::ylib::fread(STDIN, &mut c).unwrap();
    c[0]
}

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::ylib::fwrite(STDOUT, s.as_bytes()).unwrap();
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
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
