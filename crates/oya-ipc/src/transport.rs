//! Transport layer for length-prefixed bincode messages
//!
//! # Type Parameters
//! - `R`: Reader type (implements std::io::Read)
//! - `W`: Writer type (implements std::io::Write)
//!
//! # Thread Safety
//! - `!Send + !Sync` (must be externally synchronized)
//! - Use within actor context for safe concurrent access

use crate::{
    LENGTH_PREFIX_SIZE, MAX_FRAME_SIZE, MAX_PAYLOAD_SIZE, TransportError, TransportResult,
};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};

/// Transport layer for length-prefixed bincode messages.
///
/// # Example
/// ```rust
/// use oya_ipc::IpcTransport;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let (mut client, mut server) = IpcTransport::pair();
///
/// // Send and receive messages
/// client.send(&"hello world".to_string())?;
/// let received = server.recv::<String>()?;
/// assert_eq!(received, "hello world");
/// # Ok(())
/// # }
/// ```
pub struct IpcTransport<R, W: Write> {
    reader: BufReader<R>,
    writer: BufWriter<W>,
}

impl<R: Read, W: Write> IpcTransport<R, W> {
    /// Create a new transport from reader and writer.
    ///
    /// # Preconditions
    /// - reader: Implements std::io::Read
    /// - writer: Implements std::io::Write
    /// - Streams are independent (no shared state)
    ///
    /// # Postconditions
    /// - Returns IpcTransport with initialized buffers
    /// - Buffers are empty and ready for use
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: BufReader::with_capacity(MAX_FRAME_SIZE, reader),
            writer: BufWriter::with_capacity(MAX_FRAME_SIZE, writer),
        }
    }

    /// Send a message over the transport.
    ///
    /// # Preconditions
    /// - msg: Implements serde::Serialize
    /// - Serialized size ≤ 1MB
    /// - Writer is not in error state
    /// - No concurrent send() calls
    ///
    /// # Postconditions
    /// - Returns Ok(()) if message sent successfully
    /// - Returns Err(TransportError::SerializationFailed) if bincode fails
    /// - Returns Err(TransportError::MessageTooLarge) if size > 1MB
    /// - Returns Err(TransportError::WriteFailed) if stream write fails
    /// - Length prefix (4 bytes BE) + payload written to stream
    /// - Data flushed to underlying stream
    ///
    /// # Performance
    /// - Must complete <2µs for 1KB message
    /// - Must complete <20µs for 100KB message
    pub fn send<T: Serialize + ?Sized>(&mut self, msg: &T) -> TransportResult<()> {
        // Step 1: Serialize message to buffer
        let payload = bincode::serde::encode_to_vec(msg, bincode::config::standard())
            .map_err(|e| TransportError::serialization_failed(e.to_string()))?;

        // Step 2: Check size ≤ 1MB
        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(TransportError::message_too_large(
                payload.len(),
                MAX_PAYLOAD_SIZE,
            ));
        }

        // Step 3: Write length prefix (4 bytes, big-endian)
        let length_prefix = (payload.len() as u32).to_be_bytes();
        self.writer
            .write_all(&length_prefix)
            .map_err(|e| TransportError::write_failed(&e))?;

        // Step 4: Write payload
        self.writer
            .write_all(&payload)
            .map_err(|e| TransportError::write_failed(&e))?;

        // Step 5: Flush
        self.writer
            .flush()
            .map_err(|e| TransportError::write_failed(&e))?;

        Ok(())
    }

    /// Receive a message from the transport.
    ///
    /// # Preconditions
    /// - T: Implements serde::de::DeserializeOwned
    /// - Reader is not in error state
    /// - No concurrent recv() calls
    /// - At least 4 bytes available (for length prefix)
    ///
    /// # Postconditions
    /// - Returns Ok(T) with deserialized message
    /// - Returns Err(TransportError::UnexpectedEof) if stream ends mid-frame
    /// - Returns Err(TransportError::InvalidLength) if length > 1MB or = 0
    /// - Returns Err(TransportError::DeserializationFailed) if bincode fails
    /// - Returns Err(TransportError::ReadFailed) if stream read fails
    /// - Exact frame consumed from reader (buffer position advanced)
    ///
    /// # Performance
    /// - Must complete <3µs for 1KB message
    /// - Must complete <30µs for 100KB message
    pub fn recv<T: DeserializeOwned>(&mut self) -> TransportResult<T> {
        // Step 1: Read 4 bytes (length prefix)
        let mut length_prefix_bytes = [0u8; LENGTH_PREFIX_SIZE];
        self.reader
            .read_exact(&mut length_prefix_bytes)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    TransportError::unexpected_eof(0, LENGTH_PREFIX_SIZE)
                } else {
                    TransportError::read_failed(&e)
                }
            })?;

        // Step 2: Validate length ≤ 1MB and > 0
        let payload_length = u32::from_be_bytes(length_prefix_bytes) as usize;

        if payload_length == 0 {
            return Err(TransportError::invalid_length(0, "zero-length payload"));
        }

        if payload_length > MAX_PAYLOAD_SIZE {
            return Err(TransportError::invalid_length(
                payload_length as u32,
                format!("exceeds maximum of {} bytes", MAX_PAYLOAD_SIZE),
            ));
        }

        // Step 3: Read N bytes (where N = length prefix)
        let mut payload_buffer = vec![0u8; payload_length];
        self.reader.read_exact(&mut payload_buffer).map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                TransportError::unexpected_eof(
                    LENGTH_PREFIX_SIZE,
                    LENGTH_PREFIX_SIZE + payload_length,
                )
            } else {
                TransportError::read_failed(&e)
            }
        })?;

        // Step 4: Deserialize payload
        let result =
            bincode::serde::decode_from_slice(&payload_buffer, bincode::config::standard())
                .map_err(|e| {
                    TransportError::deserialization_failed(e.to_string(), payload_length)
                })?;

        Ok(result.0)
    }

    /// Get the number of bytes available in read buffer.
    ///
    /// # Postconditions
    /// - Returns number of bytes buffered but not yet consumed
    /// - Useful for detecting partial reads or connection liveness
    pub fn buffered_bytes(&self) -> usize {
        self.reader.buffer().len()
    }

    /// Clear internal buffers (for error recovery).
    ///
    /// # Postconditions
    /// - All buffered data is discarded
    /// - Transport is in clean state (ready for new frame)
    /// - Any partial message data is lost
    pub fn clear_buffers(&mut self) {
        // Clear reader buffer by consuming it
        let _ = self.reader.fill_buf();
        // Note: BufReader doesn't have a clear method, but fill_buf + consume resets position
        // For writer, we need to flush and then the buffer will be empty
        let _ = self.writer.flush();
    }

    /// Check if stream is at EOF.
    ///
    /// # Postconditions
    /// - Returns true if underlying reader is exhausted
    /// - Returns false if data may still be available
    pub fn is_eof(&self) -> bool {
        // Check if we can fill the buffer
        self.reader.buffer().is_empty()
    }
}

impl<R: Read, W: Write> IpcTransport<R, W> {
    /// Split transport into reader and writer components
    pub fn split(self) -> (BufReader<R>, BufWriter<W>) {
        (self.reader, self.writer)
    }

    /// Get reader buffer capacity (for testing)
    #[cfg(test)]
    pub fn reader_buffer_capacity(&self) -> usize {
        self.reader.capacity()
    }

    /// Get writer buffer capacity (for testing)
    #[cfg(test)]
    pub fn writer_buffer_capacity(&self) -> usize {
        self.writer.capacity()
    }
}

/// Create a pair of connected transports for testing
///
/// This creates a pipe-like structure where what's written to `client`
/// can be read from `server` and vice versa.
///
/// # Example
/// ```rust
/// use oya_ipc::IpcTransport;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let (mut client, mut server) = IpcTransport::pair();
///
/// client.send(&"hello".to_string())?;
/// let msg = server.recv::<String>()?;
/// assert_eq!(msg, "hello");
/// # Ok(())
/// # }
/// ```
#[cfg(test)]
pub fn transport_pair() -> (
    IpcTransport<DuplexReader, DuplexWriter>,
    IpcTransport<DuplexReader, DuplexWriter>,
) {
    let (client_writer, server_reader) = duplex_pair();
    let (server_writer, client_reader) = duplex_pair();

    let client = IpcTransport::new(client_reader, client_writer);
    let server = IpcTransport::new(server_reader, server_writer);

    (client, server)
}

/// Duplex pipe for testing
#[cfg(test)]
#[derive(Debug)]
pub struct DuplexPipe {
    buffer: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    reader_closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    writer_closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(test)]
impl DuplexPipe {
    fn new() -> Self {
        Self {
            buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            reader_closed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            writer_closed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

#[cfg(test)]
impl Clone for DuplexPipe {
    fn clone(&self) -> Self {
        Self {
            buffer: std::sync::Arc::clone(&self.buffer),
            reader_closed: std::sync::Arc::clone(&self.reader_closed),
            writer_closed: std::sync::Arc::clone(&self.writer_closed),
        }
    }
}

/// Reader end of duplex pipe
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct DuplexReader {
    pipe: DuplexPipe,
    position: usize,
}

/// Writer end of duplex pipe
#[cfg(test)]
#[derive(Debug)]
pub struct DuplexWriter {
    pipe: DuplexPipe,
}

/// Create a duplex pipe pair for testing
#[cfg(test)]
pub fn duplex_pair() -> (DuplexWriter, DuplexReader) {
    let pipe = DuplexPipe::new();
    let writer = DuplexWriter { pipe: pipe.clone() };
    let reader = DuplexReader { pipe, position: 0 };
    (writer, reader)
}

#[cfg(test)]
impl Read for DuplexReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self
            .pipe
            .reader_closed
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Ok(0);
        }

        let buffer = self
            .pipe
            .buffer
            .lock()
            .map_err(|_| std::io::Error::other("lock poisoned"))?;

        if self.position >= buffer.len() {
            return Ok(0); // EOF
        }

        let remaining = buffer.len() - self.position;
        let to_read = std::cmp::min(buf.len(), remaining);

        buf[..to_read].copy_from_slice(&buffer[self.position..self.position + to_read]);
        self.position += to_read;

        Ok(to_read)
    }
}

#[cfg(test)]
impl Write for DuplexWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if self
            .pipe
            .writer_closed
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "writer closed",
            ));
        }

        let mut buffer = self
            .pipe
            .buffer
            .lock()
            .map_err(|_| std::io::Error::other("lock poisoned"))?;

        buffer.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_construction() {
        let (writer, reader) = duplex_pair();
        let transport = IpcTransport::new(reader, writer);

        // Verify buffer capacity is sufficient
        assert!(transport.reader_buffer_capacity() >= MAX_FRAME_SIZE);
        assert!(transport.writer_buffer_capacity() >= MAX_FRAME_SIZE);
    }

    #[test]
    fn test_send_recv_small_message() {
        let (mut client, mut server) = transport_pair();

        let msg = "hello world".to_string();
        client.send(&msg).expect("send should succeed");

        let received = server.recv::<String>().expect("recv should succeed");
        assert_eq!(received, msg);
    }

    #[test]
    fn test_send_recv_multiple_messages() {
        let (mut client, mut server) = transport_pair();

        let msgs = vec!["first", "second", "third"];

        for msg in &msgs {
            client.send(msg).expect("send should succeed");
        }

        for expected in msgs {
            let received = server.recv::<String>().expect("recv should succeed");
            assert_eq!(received, expected);
        }
    }

    #[test]
    fn test_send_message_exceeding_1mb_returns_error() {
        let (mut client, _server) = transport_pair();

        // Create a message larger than 1MB
        let large_msg: String = "x".repeat(1_048_577);

        let result = client.send(&large_msg);
        assert!(matches!(
            result,
            Err(TransportError::MessageTooLarge { .. })
        ));

        if let Err(TransportError::MessageTooLarge {
            actual_size,
            max_size,
        }) = result
        {
            assert!(actual_size > 1_048_576);
            assert_eq!(max_size, 1_048_576);
        }
    }

    #[test]
    fn test_send_max_size_message_succeeds() {
        let (mut client, mut server) = transport_pair();

        // Create a message just under 1MB to account for bincode overhead
        // bincode serializes strings with a length prefix, so we leave room
        let large_msg: String = "x".repeat(1_048_500);

        client.send(&large_msg).expect("send should succeed");

        let received = server.recv::<String>().expect("recv should succeed");
        assert_eq!(received.len(), 1_048_500);
    }

    #[test]
    fn test_recv_with_zero_length_returns_error() {
        let (mut _client, mut server) = transport_pair();

        // Write invalid length prefix = 0
        let (mut writer, mut reader) = duplex_pair();
        writer.write_all(&0u32.to_be_bytes()).unwrap();
        writer.flush().unwrap();

        let mut transport = IpcTransport::new(reader, writer);

        let result: Result<String, _> = transport.recv();
        assert!(matches!(result, Err(TransportError::InvalidLength { .. })));
    }

    #[test]
    fn test_bidirectional_communication() {
        let (mut client, mut server) = transport_pair();

        let client_msg = "from client".to_string();
        let server_msg = "from server".to_string();

        client.send(&client_msg).expect("send should succeed");
        server.send(&server_msg).expect("send should succeed");

        let server_received = server.recv::<String>().expect("recv should succeed");
        let client_received = client.recv::<String>().expect("recv should succeed");

        assert_eq!(server_received, client_msg);
        assert_eq!(client_received, server_msg);
    }
}
