use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use woke::waker_ref;

use crate::async_rt::UserTask;

struct MyWaker;

impl woke::Woke for MyWaker {
    fn wake_by_ref(_task: &Arc<Self>) {
        // todo!()
    }
}

fn block_on(mut task: UserTask) -> <UserTask as Future>::Output {
    let mywaker = Arc::new(MyWaker);
    let waker = waker_ref(&mywaker);
    let mut context = Context::from_waker(&*waker);
    let mut task = unsafe { Pin::new_unchecked(&mut task) };
    let r = task.reactor.clone();
    let mut r = r.lock();

    loop {
        if r.is_ready(task.id) {
            match Future::poll(task.as_mut(), &mut context) {
                Poll::Ready(val) => return val,
                Poll::Pending => { r.add_task(task.id); }
            }
        } else if r.contains_task(task.id) {
            r.add_task(task.id);
        } else {
            r.register(task.id);
        }
    }
}

pub fn select<const N: usize>(tasks: Vec::<UserTask>) -> <UserTask as Future>::Output {
    let mywaker = Arc::new(MyWaker);
    let waker = waker_ref(&mywaker);
    let mut context = Context::from_waker(&*waker);
    let mut tasks = tasks.into_iter().map(|task| Box::pin(task)).collect::<Vec<Pin<Box<UserTask>>>>();

    loop {
        for task in tasks.iter_mut() {
            let r = task.reactor.clone();
            let mut r = r.lock();
            if r.is_ready(task.id) {
                match Future::poll(task.as_mut(), &mut context) {
                    Poll::Ready(val) => return val,
                    Poll::Pending => { r.add_task(task.id); }
                }
            } else if r.contains_task(task.id) {
                r.add_task(task.id);
            } else {
                r.register(task.id);
            }
        }
    }
}

pub fn join<const N: usize>(tasks: Vec<UserTask>) {
    let mywaker = Arc::new(MyWaker);
    let waker = waker_ref(&mywaker);
    let mut context = Context::from_waker(&*waker);
    let mut tasks: Vec<Pin<Box<UserTask>>> = tasks.into_iter().map(|task| Box::pin(task)).collect();

    loop {
        let mut ok = true;
        for task in tasks.iter_mut() {
            let r = task.reactor.clone();
            let mut r = r.lock();
            if r.is_finish(task.id) {
                continue;
            } else {
                ok = false;
                if r.is_ready(task.id) {
                    match Future::poll(task.as_mut(), &mut context) {
                        Poll::Ready(_) => (),
                        Poll::Pending => { r.add_task(task.id); }
                    }
                } else if r.contains_task(task.id) {
                    r.add_task(task.id);
                } else {
                    r.register(task.id);
                }
            }
        }
        if ok {
            return;
        }
    }
}

