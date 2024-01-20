use clap::{App, Arg};
use std::{
    fs::{read_dir, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    sync::Arc,
    sync::Mutex,
};
use yfs::{block_dev::BlockDevice, yfs::YeFs};

const BLOCK_SIZE: usize = 512;

#[derive(Debug)]
struct DiskImg(Mutex<File>);

impl DiskImg {
    fn new(path: String) -> Arc<Self> {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)
            .unwrap();
        f.set_len(8192 * 512).unwrap();
        Arc::new(Self(Mutex::new(f)))
    }
}

impl BlockDevice for DiskImg {
    fn read_block(&self, block_addr: yfs::constant::BlockAddr, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start(block_addr as u64 * BLOCK_SIZE as u64))
            .expect("seek error");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SIZE, "not a complete block")
    }

    fn write_block(&self, block_addr: yfs::constant::BlockAddr, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start(block_addr as u64 * BLOCK_SIZE as u64))
            .expect("seek error");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SIZE, "not a complete block")
    }
}

fn main() {
    let matches = App::new("YeFs Packer")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .help("Executable source dir(with backslash)"),
        )
        .arg(
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .help("Executable target dir(with backslash)"),
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);

    let img = DiskImg::new(format!("{target_path}yfs.img"));
    let yfs = YeFs::format(img, 8192, 1);
    let root = YeFs::root(yfs.clone());
    read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|entry| {
            let mut name_with_ext = entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .for_each(|app| {
            let mut file = File::open(format!("{target_path}{app}")).unwrap();
            let mut data = Vec::new();
            file.read_to_end(&mut data).unwrap();
            let vnode = root.create(&app).unwrap();
            vnode.write(0, &data);
        });
    let vnode = root.create("hello.js").unwrap();
    vnode.write(
        0,
        br#"
console.log('hello world')
                "#,
    );
    for entry in root.ls() {
        let name = entry.name();
        let inode = entry.inode_idx;
        println!("name = {}, inode = {:?}", name, inode);
    }
    yfs.flush();
}
