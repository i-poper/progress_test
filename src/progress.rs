use std::fmt;
use std::io::SeekFrom;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncSeek, ErrorKind, Result};
use tokio::prelude::*;
use tokio::sync::watch::{Sender, Receiver};

use crate::item::Item;

struct ProgressInner<T> {
    name: String,
    total: u64,
    size: u64,
    buf: T,
}

impl<T> fmt::Debug for ProgressInner<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgressInner")
            .field("name", &self.name)
            .field("total", &self.total)
            .field("size", &self.size)
            .finish()
    }
}

pub struct Progress<T> {
    inner: ProgressInner<T>,
    tx: Sender<Item>,
    rx: Receiver<bool>,
}

impl<T> Progress<T> {
    pub fn new(name: impl ToString, tx: Sender<Item>, rx: Receiver<bool>, buf: T) -> Self {
        let inner = ProgressInner {
            name: name.to_string(),
            total: 0,
            size: 0,
            buf,
        };
        Progress { inner, tx, rx }
    }

    fn to_item(&self, canceled: bool ) -> Item {
        Item {
            name: self.inner.name.clone(),
            total: self.inner.total,
            size: self.inner.size,
            canceled,
        }
    }
}

impl<T: AsyncRead + Unpin + Send> AsyncRead for Progress<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        Pin::new(&mut self.inner.buf).poll_read(cx, buf)
    }
}

impl<T: AsyncWrite + Unpin + Send> AsyncWrite for Progress<T> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize>> {
        let canceled = *self.rx.borrow();
        if canceled {
            let _ = self.tx.broadcast(self.to_item(canceled));
            Poll::Ready(Err(io::Error::new(ErrorKind::Interrupted, "canceled")))
        } else {
            let poll = Pin::new(&mut self.inner.buf).poll_write(cx, buf);
            if let Poll::Ready(Ok(n)) = poll {
                self.inner.size += n as u64;
                let _ = self.tx.broadcast(self.to_item(canceled));
            }
            println!("{}", self.inner.size);
            poll
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        Pin::new(&mut self.inner.buf).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        Pin::new(&mut self.inner.buf).poll_shutdown(cx)
    }
}

impl<T: AsyncSeek + Unpin + Send> AsyncSeek for Progress<T> {
    fn start_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        position: SeekFrom,
    ) -> Poll<Result<()>> {
        Pin::new(&mut self.inner.buf).start_seek(cx, position)
    }

    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<u64>> {
        Pin::new(&mut self.inner.buf).poll_complete(cx)
    }
}
