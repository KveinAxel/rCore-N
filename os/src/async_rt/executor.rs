use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::task::{Context, Poll};

use lazy_static::*;
use riscv::register::sie;
use spin::Mutex;
use woke::waker_ref;

use crate::async_rt::TaskId;

use super::task::KernelTask;

lazy_static! {
    pub static ref KERNEL_TASK_QUEUE: Arc<Mutex<Box<KernelTaskQueue>>> =
        Arc::new(
            Mutex::new(
                Box::new(
                    KernelTaskQueue {
                        queue: Vec::new()
                    }
                )
            )
        );
}

pub struct KernelTaskQueue {
    queue: Vec<Arc<KernelTask>>,
}

impl KernelTaskQueue {
    pub fn add_task(&mut self, task: KernelTask) {
        self.queue.push(Arc::new(task));
    }

    pub fn peek_task(&self) -> Option<&Arc<KernelTask>> {
        self.queue.first()
    }

    pub fn delete_task(&mut self, id: TaskId) {
        let index = self.queue.iter().position(|task| task.id == id).unwrap();
        self.queue.remove(index);
    }
}

pub fn run_until_idle() {
    loop {
        ext_int_off();
        let queue = KERNEL_TASK_QUEUE.lock();
        let task = queue.peek_task();
        ext_int_on();
        match task {
            // have any task
            Some(task) => {
                let waker = waker_ref(task);
                let mut context = Context::from_waker(&*waker);
                let ret = task.future.lock().as_mut().poll(&mut context);
                if let Poll::Ready(_) = ret {
                    let mut queue = KERNEL_TASK_QUEUE.lock();
                    queue.delete_task(task.id)
                }
            }
            None => break
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