mod device;
mod fd;
mod repl;
use std::process::exit;

use repl::Repl;
use rustyline::{DefaultEditor, Result};

fn main() -> rustyline::Result<()> {
    Repl::new()?.run();
    Ok(())
}
