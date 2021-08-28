use crate::async_rt::{AsyncClose, AsyncPipe, AsyncRead, AsyncWrite};

pub async fn sys_close(fd: usize) -> isize {
    AsyncClose::new(fd).await
}

pub async fn sys_pipe(pipe: &mut [usize]) -> isize {
    AsyncPipe::new(pipe[0] as usize).await
}

pub async fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    AsyncRead::new(fd, buffer).await
}

pub async fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    AsyncWrite::new(fd, buffer).await
}