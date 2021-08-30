use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use spin::Mutex;

use crate::async_rt::{TaskId, UserTask, REACTOR};
use crate::syscall::{syscall, SYSCALL_CLOSE, SYSCALL_PIPE, SYSCALL_READ, SYSCALL_WRITE};

pub struct AsyncClose {
    first: bool,
    task_id: usize,
    fd: usize,
}

impl AsyncClose {
    pub fn new(fd: usize) -> UserTask {
        let id = TaskId::generate();
        let future = AsyncClose {
            first: true,
            task_id: id.into(),
            fd,
        };
        UserTask {
            id,
            future: Mutex::new(Box::pin(future)),
            reactor: REACTOR.clone(),
        }
    }
}

impl Future for AsyncClose {
    type Output = isize;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.first {
            self.first = false;
            syscall(SYSCALL_CLOSE, [self.fd, self.task_id, 0, 0]);
            Poll::Pending
        } else {
            Poll::Ready(0)
        }
    }
}

pub struct AsyncRead {
    first: bool,
    task_id: usize,
    fd: usize,
    buffer: usize,
    len: usize,
}

impl AsyncRead {
    pub fn new(fd: usize, buffer: &mut [u8]) -> UserTask {
        let id = TaskId::generate();
        let future = AsyncRead {
            first: true,
            task_id: id.into(),
            fd,
            buffer: buffer.as_mut_ptr() as usize,
            len: buffer.len(),
        };
        UserTask {
            id,
            future: Mutex::new(Box::pin(future)),
            reactor: REACTOR.clone(),
        }
    }
}

impl Future for AsyncRead {
    type Output = isize;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.first {
            self.first = false;
            syscall(
                SYSCALL_READ,
                [self.fd, self.buffer, self.len, self.task_id],
            );
            Poll::Pending
        } else {
            Poll::Ready(0)
        }
    }
}

pub struct AsyncWrite {
    first: bool,
    task_id: usize,
    fd: usize,
    buffer: usize,
    len: usize,
}

impl AsyncWrite {
    pub fn new(fd: usize, buffer: &[u8]) -> UserTask {
        let id = TaskId::generate();
        let future = AsyncWrite {
            first: true,
            task_id: id.into(),
            fd,
            buffer: buffer.as_ptr() as usize,
            len: buffer.len(),
        };
        UserTask {
            id,
            future: Mutex::new(Box::pin(future)),
            reactor: REACTOR.clone(),
        }
    }
}

impl Future for AsyncWrite {
    type Output = isize;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.first {
            self.first = false;
            syscall(SYSCALL_WRITE, [self.fd, self.buffer, self.len, self.task_id]);
            Poll::Pending
        } else {
            Poll::Ready(0)
        }
    }
}

pub struct AsyncPipe {
    first: bool,
    task_id: usize,
    pipe: usize,
}

impl AsyncPipe {
    pub fn new(pipe: usize) -> UserTask {
        let id = TaskId::generate();
        let future = AsyncPipe {
            first: true,
            task_id: id.0,
            pipe,
        };
        println!("new AsyncPipe");
        UserTask {
            id,
            future: Mutex::new(Box::pin(future)),
            reactor: REACTOR.clone(),
        }
    }
}

impl Future for AsyncPipe {
    type Output = isize;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("poll");
        if self.first {
            self.first = false;
            println!("first poll, user_id: {}", self.task_id);
            syscall(SYSCALL_PIPE, [self.pipe, self.task_id, 0, 0]);
            Poll::Pending
        } else {
            Poll::Ready(0)
        }
    }
}

