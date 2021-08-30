use alloc::boxed::Box;
use core::cmp::min;
use spin::Mutex;

use crate::{
    mm::{translated_byte_buffer, translated_refmut, UserBuffer},
    task::find_task,
};
use crate::fs::{File, make_pipe};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize, user_task_id: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release Task lock manually to avoid deadlock
        drop(inner);
        if user_task_id == 0 {
            if let Ok(buffers) = translated_byte_buffer(token, buf, len) {
                match file.write(UserBuffer::new(buffers)) {
                    Ok(write_len) => write_len as isize,
                    Err(_) => -1,
                }
            } else {
                -1
            }
        } else {
            use crate::async_rt::{AsyncWrite, KERNEL_TASK_QUEUE, REACTOR, KernelTask};

            let future = AsyncWrite::new(file, buf, len, token);
            let future = Box::pin(future);
            let mut queue = KERNEL_TASK_QUEUE.lock();
            queue.add_task(KernelTask::new(REACTOR.clone(), task.getpid(), user_task_id, Mutex::new(future)));

            0
        }
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize, user_task_id: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.acquire_inner_lock();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release Task lock manually to avoid deadlock
        drop(inner);
        if user_task_id == 0 {
            if let Ok(buffers) = translated_byte_buffer(token, buf, len) {
                match file.read(UserBuffer::new(buffers)) {
                    Ok(read_len) => read_len as isize,
                    Err(_) => -1,
                }
            } else {
                -1
            }
        } else {
            use crate::async_rt::{AsyncRead, KERNEL_TASK_QUEUE, REACTOR, KernelTask};

            let future = AsyncRead::new(file, buf, len, token);
            let future = Box::pin(future);
            let mut queue = KERNEL_TASK_QUEUE.lock();
            queue.add_task(KernelTask::new(REACTOR.clone(), task.getpid(), user_task_id, Mutex::new(future)));
            0
        }
    } else {
        -1
    }
}

pub fn sys_close(fd: usize, user_task_id: usize) -> isize {
    let task = current_task().unwrap();
    if user_task_id == 0 {
        let mut inner = task.acquire_inner_lock();
        if fd >= inner.fd_table.len() {
            return -1;
        }
        if inner.fd_table[fd].is_none() {
            return -1;
        }
        inner.fd_table[fd].take();
        0
    } else {
        use crate::async_rt::AsyncClose;

        let future = AsyncClose {
            tcb: task.clone(),
            fd,
        };
        let future = Box::pin(future);
        let pid = task.pid.0;

        use crate::async_rt::{KERNEL_TASK_QUEUE, REACTOR, KernelTask};

        let mut queue = KERNEL_TASK_QUEUE.lock();
        queue.add_task(KernelTask::new(REACTOR.clone(), pid, user_task_id, Mutex::new(future)));
        0
    }
}

pub fn sys_pipe(pipe: *mut usize, user_task_id: usize) -> isize {
    debug!("sys pipe, user_id: {}", user_task_id);
    let task = current_task().unwrap();
    let token = current_user_token();

    if user_task_id == 0 {
        let mut inner = task.acquire_inner_lock();
        let (pipe_read, pipe_write) = make_pipe();
        let read_fd = inner.alloc_fd();
        inner.fd_table[read_fd] = Some(pipe_read);
        let write_fd = inner.alloc_fd();
        inner.fd_table[write_fd] = Some(pipe_write);
        *translated_refmut(token, pipe) = read_fd;
        *translated_refmut(token, unsafe { pipe.add(1) }) = write_fd;
        0
    } else {
        debug!("syscall async pipe");
        use crate::async_rt::{AsyncPipeOpen, REACTOR, KERNEL_TASK_QUEUE, KernelTask};

        let future = AsyncPipeOpen::new(task.clone(), token, pipe);
        let future = Box::pin(future);
        KERNEL_TASK_QUEUE.lock().add_task(KernelTask::new(REACTOR.clone(), task.getpid(), user_task_id, Mutex::new(future)));

        0
    }
}

pub fn sys_mailwrite(pid: usize, buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    if let Some(receive_task) = find_task(pid) {
        debug!("find task");
        if receive_task.acquire_inner_lock().is_mailbox_full() {
            return -1;
        } else if len == 0 {
            return 0;
        }

        if let Ok(buffers) = translated_byte_buffer(token, buf, min(len, 256)) {
            let socket = receive_task.create_socket();
            match socket.write(UserBuffer::new(buffers)) {
                Ok(write_len) => write_len as isize,
                Err(_) => -1,
            }
        } else {
            -1
        }
    } else {
        debug!("not find task");
        -1
    }
}

pub fn sys_mailread(buf: *mut u8, len: usize) -> isize {
    let token = current_user_token();
    let task = current_task().unwrap();
    debug!(
        "mail box empty ? {}",
        task.acquire_inner_lock().is_mailbox_empty()
    );
    if task.acquire_inner_lock().is_mailbox_empty() {
        return -1;
    } else if len == 0 {
        return 0;
    }
    let mail_box = task.acquire_inner_lock().mail_box.clone();
    if let Ok(buffers) = translated_byte_buffer(token, buf, min(len, 256)) {
        match mail_box.read(UserBuffer::new(buffers)) {
            Ok(read_len) => {
                debug!("mail read {} len", read_len);
                read_len as isize
            }
            Err(_) => -1,
        }
    } else {
        -1
    }
}
