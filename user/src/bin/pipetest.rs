#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

extern crate alloc;

use alloc::format;
use user_lib::{fclose, fork, fread, fwrite, make_pipe, time, wait};

const LENGTH: usize = 3000;
#[no_mangle]
pub fn main() -> i32 {
    // create pipes
    // parent write to child
    let down_pipe_fd = make_pipe().unwrap();
    // child write to parent
    let up_pipe_fd = make_pipe().unwrap();
    let mut random_str = [0u8; LENGTH];
    match fork() {
        user_lib::ForkResult::Parent(_) => {
            // close read end of down pipe
            fclose(down_pipe_fd[0]).unwrap();
            // close write end of up pipe
            fclose(up_pipe_fd[1]).unwrap();
            // generate a long random string
            for ch in random_str.iter_mut() {
                *ch = time() as u8;
            }
            // send it
            assert_eq!(
                fwrite(down_pipe_fd[1], &random_str).unwrap() as usize,
                random_str.len()
            );
            // close write end of down pipe
            fclose(down_pipe_fd[1]).unwrap();
            // calculate sum(parent)
            let sum: usize = random_str.iter().map(|v| *v as usize).sum::<usize>();
            println!("sum = {}(parent)", sum);
            // recv sum(child)
            let mut child_result = [0u8; 32];
            let result_len = fread(up_pipe_fd[0], &mut child_result).unwrap() as usize;
            fclose(up_pipe_fd[0]).unwrap();
            // check
            assert_eq!(
                sum,
                str::parse::<usize>(core::str::from_utf8(&child_result[..result_len]).unwrap())
                    .unwrap()
            );
            wait();
            println!("pipe_large_test passed!");
            0
        }
        user_lib::ForkResult::Child => {
            fclose(down_pipe_fd[1]).unwrap();
            // close read end of up pipe
            fclose(up_pipe_fd[0]).unwrap();
            assert_eq!(
                fread(down_pipe_fd[0], &mut random_str).unwrap() as usize,
                LENGTH
            );
            fclose(down_pipe_fd[0]).unwrap();
            let sum: usize = random_str.iter().map(|v| *v as usize).sum::<usize>();
            println!("sum = {}(child)", sum);
            let sum_str = format!("{}", sum);
            fwrite(up_pipe_fd[1], sum_str.as_bytes()).unwrap();
            fclose(up_pipe_fd[1]).unwrap();
            println!("Child process exited!");
            0
        } // // close write end of down pipe
    }
}
