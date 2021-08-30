use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::future::Future;
use core::task::{Context, Poll};

use lazy_static::*;
use riscv::register::sie;
use spin::Mutex;
use woke::waker_ref;

use crate::async_rt::TaskId;
use crate::syscall::sys_send_msg;

use super::task::KernelTask;

lazy_static! {
    pub static ref KERNEL_TASK_QUEUE: Arc<Mutex<Box<KernelTaskQueue>>> =
        Arc::new(
            Mutex::new(
                Box::new(
                    KernelTaskQueue {
                        queue: VecDeque::new()
                    }
                )
            )
        );
}

pub struct KernelTaskQueue {
    queue: VecDeque<Arc<KernelTask>>,
}

impl KernelTaskQueue {
    pub fn add_task(&mut self, task: KernelTask) {
        self.queue.push_back(Arc::new(task));
    }

    pub fn add_arc_task(&mut self, task: Arc<KernelTask>) {
        self.queue.push_back(task);
    }

    pub fn peek_task(&mut self) -> Option<Arc<KernelTask>> {
        self.queue.pop_front()
    }

    pub fn delete_task(&mut self, id: TaskId) {
        let index = self.queue.iter().position(|task| task.id == id).unwrap();
        self.queue.remove(index);
    }
}

pub fn run_until_idle() {
    loop {
        ext_int_off();
        let mut queue = KERNEL_TASK_QUEUE.lock();
        let task = queue.peek_task();
        // debug!("running, queue len: {}, task: {:?}", queue.queue.len(), task.is_none());
        ext_int_on();
        match task {
            // have any task
            Some(task) => {
                let mywaker = task.clone();
                let waker = waker_ref(&mywaker);
                let mut context = Context::from_waker(&*waker);

                let r = task.reactor.clone();
                let mut r = r.lock();

                if r.is_ready(task.id) {
                    let mut future = task.future.lock();
                    match future.as_mut().poll(&mut context) {
                        Poll::Ready(_) => {
                            // 任务完成
                            r.finish_task(task.id);
                            let msg = task.user_task_id + 1 + usize::MAX / 2;
                            sys_send_msg(task.pid, msg);
                            return
                        }
                        Poll::Pending => {
                            r.add_task(task.id);
                        }
                    }
                } else if r.contains_task(task.id) {
                    r.add_task(task.id);
                } else {
                    let mut future = task.future.lock();
                    debug!("first poll");
                    match future.as_mut().poll(&mut context) {
                        Poll::Ready(_) => {
                            // 任务完成
                            debug!("task completed");
                            let msg = task.user_task_id + 1 + usize::MAX / 2;
                            sys_send_msg(task.pid, msg);
                            return
                        }
                        Poll::Pending => {
                            r.register(task.id);
                        }
                    }
                }
            }
            None => return
        }
    }
}

/// 打开外部中断
pub fn ext_int_on() {
    unsafe {
        sie::set_sext();
    }
}

/// 关闭外部中断
pub fn ext_int_off() {
    unsafe {
        sie::clear_sext();
    }
}