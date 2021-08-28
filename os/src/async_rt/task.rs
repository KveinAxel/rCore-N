use alloc::sync::Arc;
use spin::Mutex;
use alloc::boxed::Box;
use core::task::{Poll, Context};
use core::pin::Pin;
use core::future::Future;

use super::reactor::Reactor;
use crate::syscall::sys_send_msg;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::async_rt::TaskState;

bitflags! {
    pub struct TaskResult: u32 {
        const SUCCESS = 0b1;
        const ERROR = 0b10;
    }
}

pub struct KernelTask {
    // 任务编号
    pub id: TaskId,
    // 调用进程
    pub pid: usize,
    // 调用进程的Reactor
    pub reactor: Arc<Mutex<Box<Reactor>>>,
    // 调用的任务内容
    pub future: Mutex<Pin<Box<dyn Future<Output=isize> + 'static + Send + Sync>>>, // 用UnsafeCell代替Mutex会好一点
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct TaskId(usize);

impl TaskId {
    pub(crate) fn generate() -> TaskId {
        // 任务编号计数器，任务编号自增
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        if id > usize::MAX / 2 {
            // TODO: 不让系统 Panic
            panic!("too many tasks!")
        }
        TaskId(id)
    }
}

impl KernelTask {
    pub fn new(reactor: Arc<Mutex<Box<Reactor>>>, pid: usize, fut: Mutex<Pin<Box<dyn Future<Output=isize> + 'static + Send + Sync>>>) -> Self {
        KernelTask {
            id: TaskId::generate(),
            pid,
            future: fut,
            reactor,
        }
    }

    pub fn send_msg(&self, msg: TaskResult) {
        sys_send_msg(self.pid, msg.bits as usize);
    }

    pub fn do_wake(self: &Arc<Self>) {
        todo!()
    }
}

impl Future for KernelTask {
    type Output = usize;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut r = self.reactor.lock();
        if r.is_ready(self.id) {
            *r.get_task_mut(self.id).unwrap() = TaskState::Finish;
            self.send_msg(TaskResult::SUCCESS);
            Poll::Ready(self.id.0)
        } else if r.contains_task(self.id) {
            r.add_task(self.id);
            Poll::Pending
        } else {
            r.register(self.id); // fixme
            Poll::Pending
        }
    }
}

impl woke::Woke for KernelTask {
    fn wake_by_ref(task: &Arc<Self>) {
        task.do_wake()
    }
}