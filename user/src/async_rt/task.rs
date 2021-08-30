use alloc::boxed::Box;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::task::{Context, Poll};

use spin::Mutex;

use crate::async_rt::TaskState;

use super::reactor::Reactor;

pub struct UserTask {
    // 任务编号
    pub id: TaskId,
    // 调用进程的Reactor
    pub reactor: Arc<Mutex<Box<Reactor>>>,
    // 调用的任务内容
    pub future: Mutex<Pin<Box<dyn Future<Output=isize> + 'static + Send + Sync>>>, // 用UnsafeCell代替Mutex会好一点
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct TaskId(pub(crate) usize);

impl TaskId {
    pub(crate) fn generate() -> TaskId {
        // 任务编号计数器，任务编号自增
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        if id > usize::MAX / 2 {
            // TODO: 不让系统 Panic
            panic!("too many tasks!")
        }
        TaskId(id)
    }
}

impl From<TaskId> for usize {
    fn from(task_id: TaskId) -> Self {
        task_id.0
    }
}

impl UserTask {
    pub fn new(reactor: Arc<Mutex<Box<Reactor>>>, fut: Mutex<Pin<Box<dyn Future<Output=isize> + 'static + Send + Sync>>>) -> Self {
        UserTask {
            id: TaskId::generate(),
            future: fut,
            reactor,
        }
    }

    pub fn do_wake(self: &Arc<Self>) {
        // todo
    }
}

impl Future for UserTask {
    type Output = isize;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut r = self.reactor.lock();
        if r.is_ready(self.id) {
            println!("is ready");
            *r.get_task_mut(self.id).unwrap() = TaskState::Finish;
            let mut future = self.future.lock();
            let ret = future.as_mut().poll(cx);
            match ret {
                Poll::Ready(val) => {
                    r.finish_task(self.id);
                    Poll::Ready(val)
                }
                Poll::Pending => {
                    r.add_task(self.id);
                    Poll::Pending
                }
            }
        } else if r.contains_task(self.id) {
            r.add_task(self.id);
            Poll::Pending
        } else {
            let mut f = self.future.lock();
            match f.as_mut().poll(cx) {
                Poll::Ready(val) => {
                    return Poll::Ready(0);
                }
                Poll::Pending => {
                    println!("register");
                    r.register(self.id); // fixme
                    Poll::Pending
                }
            }
        }
    }
}

impl woke::Woke for UserTask {
    fn wake_by_ref(task: &Arc<Self>) {
        task.do_wake()
    }
}

impl Drop for UserTask {
    fn drop(&mut self) {
        let r = self.reactor.clone();
        let mut r = r.lock();
        r.remove_task(self.id);
    }
}