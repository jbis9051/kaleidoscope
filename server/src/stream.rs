use crate::ipc;
use common::models::media::Media;
use std::io;
use std::io::SeekFrom;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, ReadBuf};
use tokio::time::timeout;
use tokio_util::sync::ReusableBoxFuture;
use crate::ipc::BufUnixStream;

pub struct RemoteMediaFile {
    media: Media,
    stream: Option<BufUnixStream>,

    cursor: u64,
    len: u64,

    current_task: ReusableBoxFuture<'static, (io::Result<(u64, u64, Vec<u8>)>, BufUnixStream)>,
}

impl RemoteMediaFile {
    pub fn new(len: u64, media: Media, stream: BufUnixStream) -> Self {
        
        Self {
            media,
            cursor: 0,
            len,
            stream: Some(stream),
            current_task: ReusableBoxFuture::new(async { unreachable!() }),
        }
    }

    async fn send_request(
        mut stream: BufUnixStream,
        media: Media,
        start: u64,
        end: u64,
    ) -> (io::Result<(u64, u64, Vec<u8>)>, BufUnixStream) {
        let res_size = match ipc::request_file(&mut stream, &media, start, end).await {
            Err(e) => return (Err(io::Error::new(io::ErrorKind::Other, format!("unable to execute ipc file request: {} | {}", media.path, e))), stream),
            Ok(stream) => stream,
        };
        let mut buf = vec![0; res_size as usize];
        if let Err(e) = timeout(Duration::from_secs(5), stream.reader.read_exact(&mut buf)).await {
            return (Err(io::Error::new(io::ErrorKind::Other, format!("unable to read from stream, ipc server isn't giving us what it said it would: {} | {}", media.path, e))), stream);
        }
        
        (Ok((start, end, buf)), stream)
    }

    fn start(&mut self, start: u64, end: u64) {
        let stream = self.stream.take().unwrap();
        self.current_task.set(Box::pin(Self::send_request(
            stream,
            self.media.clone(),
            start,
            end,
        )));
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<io::Result<(u64, u64, Vec<u8>)>> {
        if self.stream.is_some() {
            return Poll::Ready(Ok((u64::MAX, 0, Vec::new())));
        }
        let (data, stream) = ready!(self.current_task.poll(cx));
        self.stream = Some(stream);
        Poll::Ready(data)
    }
}

impl AsyncRead for RemoteMediaFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            match ready!(self.poll(cx))? {
                (start, end, data) if start == self.cursor => {
                    let n = Ord::min(end - start, buf.remaining() as u64);
                    buf.put_slice(&data[..n as usize]);
                    self.cursor += n;
                    break Poll::Ready(Ok(()));
                }
                _ => {}
            }
            let start = self.cursor;
            let end = Ord::min(self.cursor + buf.remaining() as u64, self.len);
            self.start(start, end);
        }
    }
}

impl AsyncSeek for RemoteMediaFile {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        self.cursor = match position {
            SeekFrom::Start(n) => n,
            SeekFrom::End(n) => self.len - n as u64,
            SeekFrom::Current(n) => self.cursor + n as u64,
        };
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Poll::Ready(Ok(self.cursor))
    }
}
