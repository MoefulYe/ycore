#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;
const LINE_PROMPT: &str = ">>> ";
const WELCOME: &str = r#"welcome to YeShell! a simple shell but work well!
>>> "#;

use alloc::string::String;
use alloc::vec::Vec;
use user_lib::{
    console::{getchar, STDIN, STDOUT},
    exec, exit, fclose, fdup, fopen, fork, make_pipe,
    types::CStr,
    wait,
    ForkResult::Child,
    OpenFlags,
};

#[derive(Debug)]
struct Cmd {
    input: Option<String>,
    output: Option<String>,
    args: Vec<String>,
}

impl Cmd {
    pub fn new(cmd: &str) -> Self {
        let mut args = cmd
            .split_whitespace()
            .map(|arg| {
                let mut arg = String::from(arg);
                arg.push('\0');
                arg
            })
            .collect::<Vec<_>>();

        // redirect input
        let input = match args
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.as_str() == "<\0")
        {
            Some((idx, _)) => args.drain(idx..=idx + 1).nth(1),
            None => None,
        };

        // redirect output
        let output = match args
            .iter()
            .enumerate()
            .find(|(_, arg)| arg.as_str() == ">\0")
        {
            Some((idx, _)) => args.drain(idx..=idx + 1).nth(1),
            None => None,
        };

        Self {
            input,
            output,
            args,
        }
    }

    fn argv(args: &Vec<String>) -> Vec<CStr> {
        let mut argv: Vec<*const u8> = args.iter().map(|arg| arg.as_ptr()).collect();
        argv.push(core::ptr::null::<u8>());
        argv
    }
}

struct CommandChain {
    cmds: Vec<Cmd>,
}

impl CommandChain {
    pub fn new(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        let chain = line.split('|').map(Cmd::new).collect();
        Some(Self { cmds: chain })
    }

    pub fn exec(self) -> Result<(), &'static str> {
        if !self.is_legal() {
            return Err("ysh: invalid command! inputs/outputs cannnot be correctly chained!");
        }
        let len = self.cmds.len();
        let pipes =
            (0..len - 1).try_fold(Vec::with_capacity(len - 1), |mut acc, _| {
                match make_pipe() {
                    Ok(pipes) => {
                        acc.push(pipes);
                        Ok(acc)
                    }
                    Err(_) => Err("ysh: failed to create pipe!"),
                }
            })?;

        for (idx, cmd) in self.cmds.into_iter().enumerate() {
            let Cmd {
                input,
                output,
                args,
            } = cmd;

            if let Child = fork() {
                if let Some(input) = input {
                    let fd = match fopen(input.as_ptr() as *const _, OpenFlags::READ) {
                        Ok(ok) => ok,
                        Err(_) => {
                            println!("ysh: failed to open file");
                            exit(-1);
                        }
                    };
                    if fclose(STDIN).is_err() {
                        println!("ysh: failed to close stdin");
                        exit(-1);
                    }
                    match fdup(fd) {
                        Ok(ok) => assert_eq!(ok, STDIN),
                        Err(_) => {
                            println!("ysh: failed to dup file descriptor");
                            exit(-1);
                        }
                    }
                    if fclose(fd).is_err() {
                        println!("ysh: failed to close file descriptor");
                        exit(-1);
                    }
                };

                if let Some(output) = output {
                    let fd = match fopen(
                        output.as_ptr() as *const _,
                        OpenFlags::WRITE | OpenFlags::CREATE,
                    ) {
                        Ok(ok) => ok,
                        Err(_) => {
                            println!("ysh: failed to open file");
                            exit(-1);
                        }
                    };
                    if fclose(STDOUT).is_err() {
                        println!("ysh: failed to close stdout");
                        exit(-1);
                    }
                    match fdup(fd) {
                        Ok(ok) => assert_eq!(ok, STDOUT),
                        Err(_) => {
                            println!("ysh: failed to dup file descriptor");
                            exit(-1);
                        }
                    }
                    if fclose(fd).is_err() {
                        println!("ysh: failed to close file descriptor");
                        exit(-1);
                    }
                };

                if idx != 0 {
                    if fclose(STDIN).is_err() {
                        println!("ysh: failed to close stdin");
                        exit(-1);
                    }
                    match fdup(unsafe { pipes.get_unchecked(idx - 1)[0] }) {
                        Ok(ok) => assert_eq!(ok, STDIN),
                        Err(_) => {
                            println!("ysh: failed to dup file descriptor");
                            exit(-1);
                        }
                    }
                }

                if idx < len - 1 {
                    if fclose(STDOUT).is_err() {
                        println!("ysh: failed to close stdout");
                        exit(-1);
                    }
                    match fdup(unsafe { pipes.get_unchecked(idx)[1] }) {
                        Ok(ok) => assert_eq!(ok, STDOUT),
                        Err(_) => {
                            println!("ysh: failed to dup file descriptor");
                            exit(-1);
                        }
                    }
                }

                for [reader, writer] in pipes {
                    if fclose(reader).is_err() {
                        println!("ysh: failed to close file descriptor");
                        exit(-1);
                    }
                    if fclose(writer).is_err() {
                        println!("ysh: failed to close file descriptor");
                        exit(-1);
                    }
                }

                let argv = &Cmd::argv(&args);
                let cmd = unsafe { args.get_unchecked(0) };
                exec(cmd, argv)
            }
        }

        for [reader, writer] in pipes {
            if fclose(reader).is_err() {
                return Err("ysh: failed to close file descriptor");
            }
            if fclose(writer).is_err() {
                return Err("ysh: failed to close file descriptor");
            }
        }

        for _ in 0..len {
            let (pid, code) = wait();
            println!("ysh: child {} exited with code {}", pid, code);
        }

        Ok(())
    }

    fn is_legal(&self) -> bool {
        if self.cmds.len() == 1 {
            true
        } else {
            self.cmds
                .iter()
                .enumerate()
                .all(|(idx, command)| match idx {
                    0 => command.output.is_none(),
                    _ if idx < self.cmds.len() - 1 => {
                        command.input.is_none() && command.output.is_none()
                    }
                    _ => command.input.is_none(),
                })
        }
    }
}

#[no_mangle]
pub fn main() -> i32 {
    println!("Rust user shell");
    let mut line: String = String::new();
    print!("{}", WELCOME);
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if let Some(commands) = CommandChain::new(&line) {
                    if let Err(msg) = commands.exec() {
                        println!("{}", msg);
                    }
                }
                line.clear();
                print!("{}", LINE_PROMPT);
            }
            BS | DL => {
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
