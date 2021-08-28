use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub use pipe::*;

use crate::fs::File;
use crate::mm::{translated_byte_buffer, UserBuffer};
use crate::task::{TaskControlBlock};

mod pipe;

pub struct AsyncFile<F: File> {
    pub file: F,
}

impl<F> AsyncFile<F> where F: File {
    pub fn new(file: F) -> AsyncFile<F> {
        AsyncFile {
            file
        }
    }

    pub async fn read() {
        todo!()
    }

    pub async fn write() {
        todo!()
    }
}

pub struct AsyncRead<F: File + Send + Sync + ?Sized> {
    pub file: Arc<F>,
    pub buf: usize,
    pub len: usize,
    pub token: usize
}

impl<F> AsyncRead<F> where F: File + Send + Sync + ?Sized {
    pub fn new(file: Arc<F>, buf: *const u8, len: usize, token: usize) -> Self {
        Self {
            file,
            buf: buf as usize,
            len,
            token
        }
    }
}

impl<F> Future for AsyncRead<F> where F: File + Send + Sync + ?Sized {
    type Output = isize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Ok(buffers) = translated_byte_buffer(self.token, self.buf as *const u8, self.len) {
            match self.file.read(UserBuffer::new(buffers)) {
                Ok(read_len) => Poll::Ready(read_len as isize),
                Err(_) => Poll::Ready(-1),
            }
        } else {
            Poll::Ready(-1)
        }
    }
}

pub struct AsyncWrite<F: File + Send + Sync + ?Sized> {
    pub file: Arc<F>,
    pub buf: usize,
    pub len: usize,
    pub token: usize
}

impl<F> AsyncWrite<F> where F: File + Send + Sync + ?Sized {
    pub fn new(file: Arc<F>, buf: *const u8, len: usize, token: usize) -> Self {
        Self {
            file,
            buf: buf as usize,
            len,
            token
        }
    }
}

impl<F> Future for AsyncWrite<F> where F: File + Send + Sync + ?Sized {
    type Output = isize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Ok(buffers) = translated_byte_buffer(self.token, self.buf as *const u8, self.len) {
            match self.file.write(UserBuffer::new(buffers)) {
                Ok(write_len) => Poll::Ready(write_len as isize),
                Err(_) => Poll::Ready(-1),
            }
        } else {
            Poll::Ready(-1)
        }
    }
}

pub struct AsyncClose {
    pub tcb: Arc<TaskControlBlock>,
    pub fd: usize,
}

impl Future for AsyncClose {
    type Output = isize;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut inner = self.tcb.acquire_inner_lock();
        if self.fd >= inner.fd_table.len() {
            return Poll::Ready(-1);
        }
        if inner.fd_table[self.fd].is_none() {
            return Poll::Ready(-1);
        }
        inner.fd_table[self.fd].take();
        Poll::Ready(0)
    }
}