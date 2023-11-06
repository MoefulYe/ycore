use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    sync::{Arc, Mutex},
};

use yfs::{block_dev::BlockDevice, constant::BLOCK_SIZE};

use crate::error::Result;

#[derive(Debug)]
pub struct DiskFileImg(Mutex<File>);

impl DiskFileImg {
    pub fn new(file: &str) -> Result<DiskFileImg> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file)?;
        f.set_len(BLOCK_SIZE as u64 * 8192)?;
        Ok(DiskFileImg(Mutex::new(f)))
    }
}

impl BlockDevice for DiskFileImg {
    fn read_block(&self, block_addr: yfs::constant::BlockAddr, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start(block_addr as u64 * BLOCK_SIZE as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE, "Not a complete block!");
    }

    fn write_block(&self, block_addr: yfs::constant::BlockAddr, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start(block_addr as u64 * BLOCK_SIZE as u64))
            .expect("Error when seeking!");
        assert_eq!(
            file.write(buf).unwrap(),
            BLOCK_SIZE,
            "Not a complete block!"
        );
    }
}
