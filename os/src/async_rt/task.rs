use alloc::boxed::Box;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};

use spin::Mutex;

use crate::async_rt::TaskState;
use crate::syscall::sys_send_msg;

use super::reactor::Reactor;

pub struct KernelTask {
    // 任务编号
    pub id: TaskId,
    // 调用进程
    pub pid: usize,
    // 调用进程的Reactor
    pub reactor: Arc<Mutex<Box<Reactor>>>,
    // 用户任务id
    pub user_task_id: usize,
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
    pub fn new(reactor: Arc<Mutex<Box<Reactor>>>, pid: usize, user_task_id: usize, fut: Mutex<Pin<Box<dyn Future<Output=isize> + 'static + Send + Sync>>>) -> Self {
        KernelTask {
            id: TaskId::generate(),
            pid,
            user_task_id,
            future: fut,
            reactor,
        }
    }

    pub fn send_msg(&self, msg: usize) {
        sys_send_msg(self.pid, msg);
    }

    pub fn do_wake(self: &Arc<Self>) {
        // todo!()
    }
}

impl Future for KernelTask {
    type Output = usize;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        debug!("poll kernel task");
        let mut r = self.reactor.lock();
        if r.is_ready(self.id) {
            *r.get_task_mut(self.id).unwrap() = TaskState::Finish;
            // 发送用户态中断
            assert!(self.user_task_id <= usize::MAX / 2);
            let msg = self.user_task_id + 1 + usize::MAX / 2;
            self.send_msg(msg);
            Poll::Ready(self.id.0)
        } else if r.contains_task(self.id) {
            r.add_task(self.id);
            Poll::Pending
        } else {
            let mut f = self.future.lock();
            debug!("first poll for future in KernelTask");
            match f.as_mut().poll(cx) {
                Poll::Ready(_) => Poll::Ready(0),
                Poll::Pending => {
                    r.register(self.id); // fixme
                    Poll::Pending
                }
            }

        }
    }
}

impl woke::Woke for KernelTask {
    fn wake_by_ref(task: &Arc<Self>) {
        task.do_wake()
    }
}

impl Drop for KernelTask {
    fn drop(&mut self) {
        let r = self.reactor.clone();
        let mut r = r.lock();

        r.remove_task(self.id);
    }
}