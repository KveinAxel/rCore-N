use alloc::sync::{Arc, Weak};

use spin::Mutex;

use crate::fs::{Pipe, make_pipe};
use crate::mm::{UserBuffer, translated_refmut};
use crate::task::{suspend_current_and_run_next, current_task, TaskControlBlock};
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct AsyncPipeOpen {
    task: Arc<TaskControlBlock>,
    token: usize,
    pipe: *mut usize
}

unsafe impl Send for AsyncPipeOpen {}
unsafe impl Sync for AsyncPipeOpen {}

impl AsyncPipeOpen {
    pub fn new(task: Arc<TaskControlBlock>, token: usize, pipe: *mut usize) -> Self {
        Self {
            task,
            token,
            pipe
        }
    }
}

impl Future for AsyncPipeOpen {
    type Output = isize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.task.acquire_inner_lock();
        let (pipe_read, pipe_write) = make_pipe();
        let read_fd = inner.alloc_fd();
        inner.fd_table[read_fd] = Some(pipe_read);
        let write_fd = inner.alloc_fd();
        inner.fd_table[write_fd] = Some(pipe_write);
        *translated_refmut(self.token, self.pipe) = read_fd;
        *translated_refmut(self.token, unsafe { self.pipe.add(1) }) = write_fd;
        Poll::Ready(0)
    }
}