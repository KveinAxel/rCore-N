use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use alloc::sync::Arc;
use crate::task::TaskControlBlock;

pub struct AsyncClose {
    pub tcb: Arc<TaskControlBlock>,
    pub fd: usize
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