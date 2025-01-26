use std::io::{BufRead, Cursor, Read, Seek};

#[derive(Debug, Clone)]
pub struct SeekRequest {
    url: String,
    cursor: Option<Cursor<Vec<u8>>>,
}

impl AsRef<str> for SeekRequest {
    fn as_ref(&self) -> &str {
        &self.url
    }
}

impl SeekRequest {
    pub fn new(url: String) -> Self {
        Self { url, cursor: None }
    }

    pub fn load(&mut self) -> std::io::Result<()> {
        if self.cursor.is_none() {
            let res = reqwest::blocking::get(&self.url).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("request: {e:?}"))
            })?;
            let res = res.bytes().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("response: {e:?}"))
            })?;
            let mut buf = Vec::with_capacity(res.len());
            buf.extend(res);
            self.cursor = Some(Cursor::new(buf));
        }
        Ok(())
    }
}
impl Seek for SeekRequest {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        tracing::debug!("seek {} {pos:?}", self.url);
        self.load()?;
        self.cursor.as_mut().unwrap().seek(pos)
    }
}

impl Read for SeekRequest {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.load()?;
        self.cursor.as_mut().unwrap().read(buf)
    }
}

impl BufRead for SeekRequest {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        tracing::debug!("fill_buf {}", self.url);
        self.load()?;
        self.cursor.as_mut().unwrap().fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        tracing::debug!("consume {}", self.url);
        self.cursor.as_mut().unwrap().consume(amt);
    }
}
