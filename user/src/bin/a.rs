#![no_std]
#![no_main]
#![feature(asm)]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering::Relaxed};
use riscv::register::uie;
use user_lib::async_rt::{sys_pipe, sys_close, sys_read, sys_write, block_on};
use user_lib::{init_user_trap, getpid};

static IS_TIMEOUT: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub fn main() -> i32 {
    println!("[async syscall] from pid: {}", getpid());
    let init_res = init_user_trap();
    println!(
        "[async syscall] trap init result: {:#x}, now using timer to sleep",
        init_res
    );
    unsafe {
        uie::set_usoft();
    }
    let mut pipe: [usize; 2] = [0; 2];
    block_on(sys_pipe(&mut pipe));
    println!("[async syscall] pipe fd is {} and {}", pipe[0], pipe[1]);
    0
}
