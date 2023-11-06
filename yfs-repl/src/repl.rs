use std::sync::Arc;

use rustyline::DefaultEditor;
use yfs::{block_dev::BlockDevice, yfs::YeFs};

use crate::device::DiskFileImg;

pub struct Repl {
    rl: DefaultEditor,
}

impl Repl {
    pub fn new() -> rustyline::Result<Self> {
        let rl = DefaultEditor::new()?;
        Ok(Self { rl })
    }

    pub fn load_device(&mut self) -> Arc<dyn BlockDevice> {
        let path = self
            .rl
            .readline("Input the path to the disk image> ")
            .unwrap();
        Arc::new(DiskFileImg::new(&path).unwrap())
    }

    pub fn load_fs(&mut self, device: &Arc<dyn BlockDevice>) -> Arc<YeFs> {
        match YeFs::load(device.clone()) {
            Some(fs) => fs,
            None => {
                println!("unvalid file system on device! format it first!");
                match self.rl.readline("format it? (y/n)> ").unwrap().as_str() {
                    "y" => YeFs::format(device.clone(), 4096, 1),
                    _ => {
                        println!("bye!");
                        std::process::exit(0);
                    }
                }
            }
        }
    }

    pub fn run(&mut self) {
        println!("Welcome to the YeFS REPL!\n");
        let device = self.load_device();
        let fs = self.load_fs(&device);
    }
}
