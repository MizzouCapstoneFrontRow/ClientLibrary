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


/// Not cancel-safe (may lose data)
pub async fn read_jpeg_with_buf(mut buf: Vec<u8>, mut stream: impl AsyncBufRead + Unpin) -> Result<Vec<u8>, ReadJpegError> {
    buf.clear();
    while !buf.ends_with(b"\xff\xd9") { // JPEG end-of-image marker
        match stream.read_until(0xd9, &mut buf).await {
            Ok(0) => return Err(ReadJpegError::EOF),
            Err(err) => return Err(ReadJpegError::IOError(err)),
            Ok(_) => {}
        }
    }
    Ok(buf)
}

/// Not cancel-safe (may lose data)
pub async fn read_jpeg(stream: impl AsyncBufRead + Unpin) -> Result<Vec<u8>, ReadJpegError> {
    read_jpeg_with_buf(Vec::with_capacity(8192), stream).await
}
