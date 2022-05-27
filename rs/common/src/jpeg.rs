use std::sync::Arc;

use buffer_cache::{BufferCache, BufferWrapper};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

#[derive(Debug)]
pub enum ReadJpegError {
    IOError(std::io::Error),
    EOF,
}

impl std::fmt::Display for ReadJpegError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadJpegError::IOError(err) =>
                write!(f, "IO error reading JPEG: {err}"),
            ReadJpegError::EOF =>
                write!(f, "reached EOF before reading JPEG"),
        }
    }
}

impl From<std::io::Error> for ReadJpegError {
    fn from(err: std::io::Error) -> Self {
        ReadJpegError::IOError(err)
    }
}

impl std::error::Error for ReadJpegError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ReadJpegError::IOError(err) => Some(err),
            ReadJpegError::EOF => None,
        }
    }
}

lazy_static::lazy_static!{
    static ref JPEG_BUFFER_CACHE: Arc<BufferCache<Vec<u8>>> = BufferCache::new(16384);
}

pub type ImageData = BufferWrapper<Vec<u8>>;

/// Cancel-safe (can run again to continue)
/// Does not clear buf before reading
pub async fn read_jpeg_into_buf(buf: &mut Vec<u8>, mut stream: impl AsyncBufRead + Unpin) -> Result<(), ReadJpegError> {
    loop {
        match stream.read_until(0xd9, buf).await {
            Ok(0) => return Err(ReadJpegError::EOF),
            Err(err) => return Err(ReadJpegError::IOError(err)),
            Ok(_) => {}
        }
        if buf.ends_with(b"\xff\xd9") { break; }
    }
    Ok(())
}

/// Not cancel-safe (may lose data)
pub async fn read_jpeg(stream: impl AsyncBufRead + Unpin) -> Result<ImageData, ReadJpegError> {
    let mut buf = JPEG_BUFFER_CACHE.get_buffer();
    read_jpeg_into_buf(&mut buf, stream).await?;
    Ok(buf)
}
