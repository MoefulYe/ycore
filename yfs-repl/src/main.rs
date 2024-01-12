mod device;
mod error;
pub mod fd;
pub mod repl;


use error::Result;
use repl::Repl;

fn main() -> Result<()> {
    Repl::new()?.run();
    Ok(())
}
