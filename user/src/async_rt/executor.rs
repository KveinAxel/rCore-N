use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use woke::waker_ref;
use riscv::register::uie;

struct MyWaker;

impl woke::Woke for MyWaker {
    fn wake_by_ref(_task: &Arc<Self>) {
        // todo!()
    }
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    let mywaker = Arc::new(MyWaker);
    let waker = waker_ref(&mywaker);
    let mut context = Context::from_waker(&*waker);
    let mut future = Box::pin(future);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Pending => {
                // println!("pending")
            }
            Poll::Ready(val) => return val
        }
    }
}

pub fn select<F: Future>(futures: Vec::<F>) -> F::Output {
    let mywaker = Arc::new(MyWaker);
    let waker = waker_ref(&mywaker);
    let mut context = Context::from_waker(&*waker);

    let mut futures = futures
        .into_iter()
        .map(|future| Box::pin(future))
        .collect::<Vec<Pin<Box<F>>>>();

    loop {
        for future in futures.iter_mut() {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(val) => return val,
                Poll::Pending => {}
            }
        }
    }
}

pub fn join<F: Future>(futures: Vec<F>) {
    let mywaker = Arc::new(MyWaker);
    let waker = waker_ref(&mywaker);
    let mut context = Context::from_waker(&*waker);

    let mut futures: Vec<Pin<Box<F>>> = futures.into_iter().map(|future| Box::pin(future)).collect();
    let mut ok = Vec::new();

    for _ in futures.iter() {
        ok.push(false);
    }
    let mut cnt = 0;
    let tot = futures.len();
    loop {
        for (i, future) in futures.iter_mut().enumerate() {
            if !ok[i] {
                match future.as_mut().poll(&mut context) {
                    Poll::Ready(_) => {
                        ok[i] = true;
                        cnt += 1;
                        if cnt == tot {
                            return;
                        }
                    }
                    Poll::Pending => {}
                }
            }
        }
    }
}

