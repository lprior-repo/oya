//! Test helpers for Oya IPC

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

/// In-memory duplex pipe for testing
pub struct TestPipe {
    buffer: Arc<Mutex<Vec<u8>>>,
    read_pos: Arc<Mutex<usize>>,
}

impl TestPipe {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            read_pos: Arc::new(Mutex::new(0)),
        }
    }
}

impl Clone for TestPipe {
    fn clone(&self) -> Self {
        Self {
            buffer: Arc::clone(&self.buffer),
            read_pos: Arc::clone(&self.read_pos),
        }
    }
}

pub struct TestReader {
    pipe: TestPipe,
}

pub struct TestWriter {
    pipe: TestPipe,
}

impl TestReader {
    pub fn new(pipe: TestPipe) -> Self {
        Self { pipe }
    }
}

impl TestWriter {
    pub fn new(pipe: TestPipe) -> Self {
        Self { pipe }
    }
}

impl Read for TestReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut buffer = self.pipe.buffer.lock().unwrap();
        let mut pos = self.pipe.read_pos.lock().unwrap();

        let available = buffer.len().saturating_sub(*pos);
        if available == 0 {
            return Ok(0);
        }

        let to_read = available.min(buf.len());
        buf[..to_read].copy_from_slice(&buffer[*pos..*pos + to_read]);
        *pos += to_read;

        Ok(to_read)
    }
}

impl Write for TestWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut buffer = self.pipe.buffer.lock().unwrap();
        buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Create a test pipe pair
pub fn test_pipe_pair() -> (TestWriter, TestReader) {
    let pipe = TestPipe::new();
    (TestWriter::new(pipe.clone()), TestReader::new(pipe))
}
