use riscv::register::{ucause, uepc, uip, uscratch, ustatus::Ustatus, utval, sie};
use rv_plic::PLIC;
use crate::async_rt::{REACTOR, TaskId};

pub const PAGE_SIZE: usize = 0x1000;
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
pub const USER_TRAP_BUFFER: usize = TRAP_CONTEXT - PAGE_SIZE;

pub const PLIC_BASE: usize = 0xc00_0000;
pub const PLIC_PRIORITY_BIT: usize = 3;

pub type Plic = PLIC<PLIC_BASE, PLIC_PRIORITY_BIT>;

pub fn hart_id() -> usize {
    let hart_id: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) hart_id);
    }
    hart_id
}

pub fn get_context(hart_id: usize, mode: char) -> usize {
    const MODE_PER_HART: usize = 3;
    hart_id * MODE_PER_HART
        + match mode {
        'M' => 0,
        'S' => 1,
        'U' => 2,
        _ => panic!("Wrong Mode"),
    }
}

#[repr(C)]
pub struct UserTrapContext {
    pub x: [usize; 32],
    pub ustatus: Ustatus,
    pub uepc: usize,
    pub utvec: usize,
    pub uie: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct UserTrapRecord {
    pub cause: usize,
    pub message: usize,
}

global_asm!(include_str!("trap.asm"));

#[linkage = "weak"]
#[no_mangle]
pub fn user_trap_handler(cx: &mut UserTrapContext) -> &mut UserTrapContext {
    let ucause = ucause::read();
    let utval = utval::read();
    match ucause.cause() {
        ucause::Trap::Interrupt(ucause::Interrupt::UserSoft) => {
            let trap_record_num = uscratch::read();
            println!("trap_record_num: {}", trap_record_num);
            let mut head_ptr = USER_TRAP_BUFFER as *const UserTrapRecord;
            for _ in 0..trap_record_num {
                unsafe {
                    let trap_record = *head_ptr;
                    let cause = trap_record.cause;
                    let msg = trap_record.message;
                    if cause & 0xF == 0 {
                        // "real" soft interrupt
                        let pid = cause >> 4;
                        soft_intr_handler(pid, msg);
                    } else if ucause::Interrupt::from(cause) == ucause::Interrupt::UserExternal {
                        let irq = trap_record.message as u16;
                        ext_intr_handler(irq, true);
                        Plic::complete(get_context(hart_id(), 'U'), irq);
                    } else if ucause::Interrupt::from(cause) == ucause::Interrupt::UserTimer {
                        timer_intr_handler(msg);
                    }
                    head_ptr = head_ptr.offset(1);
                }
            }
            unsafe {
                uip::clear_usoft();
            }
        }
        ucause::Trap::Interrupt(ucause::Interrupt::UserExternal) => {
            while let Some(irq) = Plic::claim(get_context(hart_id(), 'U')) {
                ext_intr_handler(irq, false);
                Plic::complete(get_context(hart_id(), 'U'), irq);
            }
            // println!("[user trap] user external finished");
        }
        ucause::Trap::Interrupt(ucause::Interrupt::UserTimer) => {
            timer_intr_handler(0);
            unsafe {
                uip::clear_utimer();
            }
        }
        _ => {
            println!(
                "Unsupported trap {:?}, utval = {:#x}, uepc = {:#x}!",
                ucause.cause(),
                utval,
                uepc::read()
            );
        }
    }
    cx
}

#[linkage = "weak"]
#[no_mangle]
pub fn ext_intr_handler(irq: u16, is_from_kernel: bool) {
    println!(
        "[user trap default] user external interrupt, irq: {}, is_from_kernel: {}",
        irq, is_from_kernel
    );
}

#[linkage = "weak"]
#[no_mangle]
pub fn soft_intr_handler(pid: usize, msg: usize) {
    println!(
        "[user trap default] user software interrupt, pid: {}, msg: {:#x}",
        pid, msg
    );

    // msg最高位为1表示是异步系统调用的用户态中断返回
    if msg > usize::MAX / 2 {
        let task_id= msg - usize::MAX / 2 - 1;
        let mut r = REACTOR.lock();
        r.wake(TaskId(task_id));
    }
}

#[linkage = "weak"]
#[no_mangle]
pub fn timer_intr_handler(time_us: usize) {
    println!(
        "[user trap default] user timer interrupt, time (us): {}",
        time_us
    );
}
