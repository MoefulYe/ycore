use std::process::exit;
use std::sync::Arc;

use crate::fd::{self, Fd, OpenFlags};
use crate::{device::DiskFileImg, error::Result};
use anyhow::{anyhow, Ok};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use yfs::{block_dev::BlockDevice, vfs::Vnode, yfs::YeFs};

pub struct Repl {
    rl: DefaultEditor,
    device: Arc<dyn BlockDevice>,
    fs: Arc<YeFs>,
    fd_table: fd::Table,
    root: Arc<Vnode>,
    cwd: Arc<Vnode>,
    path: Vec<String>,
    promot: String,
}

impl Repl {
    pub fn new() -> Result<Self> {
        println!("welcome to the YeFs repl! powered by ashenye");
        let mut rl = DefaultEditor::new()?;
        let device = Self::load_device(&mut rl)?;
        let fs = Self::load_fs(&mut rl, &device)?;
        let fd_table = fd::Table::new();
        let root = YeFs::root(fs.clone());
        let cwd = root.clone();
        Ok(Self {
            rl,
            device,
            fs,
            fd_table,
            root,
            cwd,
            path: vec![],
            promot: "/ > ".to_owned(),
        })
    }

    fn load_device(rl: &mut DefaultEditor) -> Result<Arc<dyn BlockDevice>> {
        match rl.readline("input the path where the disk imgage is located> ") {
            rustyline::Result::Ok(path) => {
                println!("loading disk image from {}...", path.trim());
                Ok(Arc::new(DiskFileImg::new(path.trim())?))
            }
            Err(err) => Err(anyhow!("readline error: {}", err)),
        }
    }

    fn load_fs(rl: &mut DefaultEditor, device: &Arc<dyn BlockDevice>) -> Result<Arc<YeFs>> {
        println!("loading file system from device...");
        match YeFs::load(device.clone()) {
            Some(fs) => {
                println!("file system loaded!");
                Ok(fs)
            }
            None => match rl.readline("unvalid file system! format it or exit? (y/n)> ") {
                rustyline::Result::Ok(input) => {
                    if input == "y" {
                        println!("formatting...");
                        Ok(YeFs::format(device.clone(), 4096, 1))
                    } else {
                        println!("bye!");
                        exit(0);
                    }
                }
                Err(err) => Err(anyhow!("readline error: {}", err)),
            },
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.rl.readline(&self.promot) {
                rustyline::Result::Ok(line) => {
                    if let Err(err) = self.exec(&line) {
                        println!("error: {}", err);
                    }
                }
                Err(ReadlineError::Io(err)) => {
                    println!("readline error: {}", err);
                    exit(1);
                }
                Err(ReadlineError::Eof | ReadlineError::Interrupted) => {
                    self.exit();
                }
                Err(err) => {
                    println!("readline error: {}", err);
                    exit(1);
                }
            }
        }
    }

    pub fn device(&self) -> Arc<dyn BlockDevice> {
        self.device.clone()
    }

    pub fn fs(&self) -> Arc<YeFs> {
        self.fs.clone()
    }

    pub fn fd_table(&self) -> &fd::Table {
        &self.fd_table
    }

    pub fn root(&self) -> Arc<Vnode> {
        self.root.clone()
    }

    pub fn cwd(&self) -> Arc<Vnode> {
        self.cwd.clone()
    }

    fn exec(&mut self, line: &str) -> Result<()> {
        fn matched<'a>(line: &'a str, prefix: &'a str) -> Option<&'a str> {
            if line.starts_with(prefix) {
                Some(line.trim_start_matches(prefix).trim())
            } else {
                None
            }
        }

        fn matched_noarg(line: &str, command: &str) -> Result<bool> {
            if line.starts_with(command) {
                if line.trim() == command {
                    Ok(true)
                } else {
                    Err(anyhow!("too many arguments"))
                }
            } else {
                Ok(false)
            }
        }

        let line = line.trim();
        if let Some(leftover) = matched(line, "cd") {
            let path = self.parse_cd(leftover)?;
            self.cd(path)
        } else if matched_noarg(line, "pwd")? {
            self.pwd()
        } else if matched_noarg(line, "ls")? {
            self.ls()
        } else if let Some(leftover) = matched(line, "close") {
            let fd = self.parse_close(leftover)?;
            self.close(fd)
        } else if let Some(leftover) = matched(line, "open") {
            let (name, flags) = self.parse_open(leftover)?;
            self.open(name, flags)
        } else if let Some(leftover) = matched(line, "read") {
            let (fd, size) = self.parse_read(leftover)?;
            self.read(fd, size)
        } else if let Some(leftover) = matched(line, "write") {
            let fd = self.parse_write(leftover)?;
            self.write(fd)
        } else if let Some(leftover) = matched(line, "mkdir") {
            let name = self.parse_mkdir(leftover)?;
            self.mkdir(name)
        } else if let Some(leftover) = matched(line, "rm") {
            let name = self.parse_rm(leftover)?;
            self.rm(name)
        } else if let Some(leftover) = matched(line, "create") {
            let name = self.parse_create(leftover)?;
            self.create(name)
        } else if let Some(leftover) = matched(line, "seek") {
            let (fd, step) = self.parse_seek(leftover)?;
            self.seek(fd, step)
        } else if matched_noarg(line, "flush")? {
            self.flush()
        } else if matched_noarg(line, "exit")? {
            self.exit()
        } else {
            Err(anyhow!("unknown command"))?
        }
    }

    fn parse_cd<'a>(&mut self, leftover: &'a str) -> Result<&'a str> {
        let mut split = leftover.split_whitespace();
        let path = split.next().ok_or_else(|| anyhow!("missing path"))?;
        if split.next().is_some() {
            Err(anyhow!("too many arguments"))
        } else {
            Ok(path)
        }
    }

    fn cd(&mut self, mut path: &str) -> Result<()> {
        let backup = (self.path.clone(), self.cwd.clone());

        path = if path.starts_with('/') {
            self.path.clear();
            self.cwd = self.root();
            &path[1..]
        } else {
            path
        };

        let result = || -> Result<()> {
            for name in path.split('/') {
                if name.is_empty() {
                    continue;
                }

                match self.cwd.dir_find(name) {
                    Some(vnode) => {
                        if vnode.is_file() {
                            Err(anyhow!("`{name}` is not a directory"))?;
                        } else {
                            self.cwd = vnode;
                            match name {
                                "." => {}
                                ".." => {
                                    self.path.pop();
                                }
                                name => self.path.push(name.to_owned()),
                            }
                        }
                    }
                    None => Err(anyhow!("not such directory"))?,
                }
            }
            Ok(())
        }();

        match result {
            anyhow::Result::Ok(_) => {
                let path = self.path.join("/");
                self.promot = format!("/{path} > ");
                Ok(())
            }
            Err(err) => {
                self.path = backup.0;
                self.cwd = backup.1;
                Err(err)
            }
        }
    }

    fn pwd(&self) -> Result<()> {
        println!("/{}", self.path.join("/"));
        Ok(())
    }

    fn ls(&self) -> Result<()> {
        let to_print = self
            .cwd
            .ls()
            .iter()
            .map(|entry| entry.name().to_owned())
            .collect::<Vec<_>>()
            .join(" ");
        println!("{}", to_print);
        Ok(())
    }

    fn parse_close(&mut self, leftover: &str) -> Result<Fd> {
        let mut split = leftover.split_whitespace();
        let fd: Fd = split
            .next()
            .map(|fd| fd.parse())
            .ok_or_else(|| anyhow!("missing fd"))??;
        if split.next().is_some() {
            Err(anyhow!("too many arguments"))
        } else {
            Ok(fd)
        }
    }

    fn close(&mut self, fd: Fd) -> Result<()> {
        self.fd_table.close(fd)?;
        println!("close fd {fd}");
        Ok(())
    }

    fn parse_open<'a>(&mut self, leftover: &'a str) -> Result<(&'a str, OpenFlags)> {
        let mut split = leftover.split_whitespace();
        let name = split.next().ok_or_else(|| anyhow!("missing name"))?;
        let flags = split
            .next()
            .map(|flags| {
                flags.chars().fold(
                    anyhow::Result::Ok(OpenFlags::empty()),
                    |acc, ch| match acc {
                        anyhow::Result::Ok(flags) => match ch {
                            'r' => anyhow::Result::Ok(flags | OpenFlags::READ),
                            'w' => anyhow::Result::Ok(flags | OpenFlags::WRITE),
                            'a' => anyhow::Result::Ok(flags | OpenFlags::APPEND),
                            'c' => anyhow::Result::Ok(flags | OpenFlags::CREATE),
                            't' => anyhow::Result::Ok(flags | OpenFlags::TRUNC),
                            _ => Err(anyhow!("invalid open flags")),
                        },
                        acc => acc,
                    },
                )
            })
            .ok_or_else(|| anyhow!("missing flags"))??;

        if split.next().is_some() {
            Err(anyhow!("too many arguments"))?
        } else {
            Ok((name, flags))
        }
    }

    fn open(&mut self, name: &str, flags: OpenFlags) -> Result<()> {
        let this = unsafe { &mut *(self as *mut _) };
        let fd = self.fd_table.open(name, flags, this)?;
        println!("open `{name}` with fd {fd}");
        Ok(())
    }

    fn parse_read(&mut self, leftover: &str) -> Result<(Fd, u32)> {
        let mut split = leftover.split_whitespace();
        let fd: Fd = split
            .next()
            .map(|fd| fd.parse())
            .ok_or_else(|| anyhow!("missing fd"))??;
        let size: u32 = split
            .next()
            .map(|size| size.parse())
            .ok_or_else(|| anyhow!("missing size"))??;

        if split.next().is_some() {
            Err(anyhow!("too many arguments"))?
        } else {
            Ok((fd, size))
        }
    }

    fn read(&mut self, fd: Fd, size: u32) -> Result<()> {
        let mut buf = vec![0; size as usize];
        let nread = self.fd_table.read(fd, &mut buf)?;
        let s = String::from_utf8_lossy(&buf[..nread as usize]);
        println!("{s}\n");
        println!("read {} bytes from fd {fd}", nread);
        Ok(())
    }

    fn parse_write(&mut self, leftover: &str) -> Result<Fd> {
        let mut split = leftover.split_whitespace();
        let fd: Fd = split
            .next()
            .map(|fd| fd.parse())
            .ok_or_else(|| anyhow!("missing fd"))??;

        if split.next().is_some() {
            Err(anyhow!("too many arguments"))?
        } else {
            Ok(fd)
        }
    }

    fn write(&mut self, fd: Fd) -> Result<()> {
        let mut buf = String::new();
        while let rustyline::Result::Ok(line) = self.rl.readline("here you write> ") {
            buf.push_str(&line);
            buf.push('\n');
        }
        let nwrite = self.fd_table.write(fd, buf.as_bytes())?;
        println!("write {} bytes to fd {fd}", nwrite);
        Ok(())
    }

    fn parse_mkdir<'a>(&mut self, leftover: &'a str) -> Result<&'a str> {
        let mut split = leftover.split_whitespace();
        let name = split.next().ok_or_else(|| anyhow!("missing name"))?;
        if split.next().is_some() {
            Err(anyhow!("too many arguments"))
        } else {
            Ok(name)
        }
    }

    // 参数中不能包含路径分隔符
    fn mkdir(&mut self, name: &str) -> Result<()> {
        self.cwd
            .mkdir(name)
            .map_err(|_| anyhow!("`{name}` has existed"))?;
        println!("mkdir `{name}`", name = name);
        Ok(())
    }

    fn parse_rm<'a>(&mut self, leftover: &'a str) -> Result<&'a str> {
        let mut split = leftover.split_whitespace();
        let name = split.next().ok_or_else(|| anyhow!("missing name"))?;
        if split.next().is_some() {
            Err(anyhow!("too many arguments"))
        } else {
            Ok(name)
        }
    }

    fn rm(&mut self, name: &str) -> Result<()> {
        self.cwd
            .dir_rm(name)
            .map_err(|_| anyhow!("cannot remove `{name}`: No such file or directory"))?;
        println!("remove `{name}`", name = name);
        Ok(())
    }

    fn parse_create<'a>(&mut self, leftover: &'a str) -> Result<&'a str> {
        let mut split = leftover.split_whitespace();
        let name = split.next().ok_or_else(|| anyhow!("missing name"))?;
        if split.next().is_some() {
            Err(anyhow!("too many arguments"))
        } else {
            Ok(name)
        }
    }

    fn create(&mut self, name: &str) -> Result<()> {
        self.cwd
            .create(name)
            .map_err(|_| anyhow!("`{name}` has existed"))?;
        println!("create `{name}`", name = name);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.fs.flush();
        println!("flush");
        Ok(())
    }

    fn parse_seek(&mut self, leftover: &str) -> Result<(Fd, i32)> {
        let mut split = leftover.split_whitespace();
        let fd = split
            .next()
            .ok_or_else(|| anyhow!("missing fd"))?
            .parse::<Fd>()?;
        let step = split
            .next()
            .ok_or_else(|| anyhow!("missing offset"))?
            .parse::<i32>()?;
        if split.next().is_some() {
            Err(anyhow!("too many arguments"))
        } else {
            Ok((fd, step))
        }
    }

    fn seek(&mut self, fd: Fd, step: i32) -> Result<()> {
        self.fd_table.seek(fd, step)
    }

    fn exit(&mut self) -> ! {
        self.fs.flush();
        println!("bye!");
        exit(0);
    }
}
