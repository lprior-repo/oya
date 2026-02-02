//! High-throughput streaming text support
//!
//! Ring buffer implementation for streaming process output.
//! Optimized for:
//! - Zero allocation during streaming
//! - Line indexing for virtual scrolling
//! - Chunked emission (4KB) for efficient IPC
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                   StreamBuffer (64KB)                    │
//! ├─────────────────────────────────────────────────────────┤
//! │                                                         │
//! │  ┌─────────────────────────────────────────────────┐   │
//! │  │  Ring Buffer: [........ABCDEFGHIJ..........]    │   │
//! │  │                       ↑         ↑               │   │
//! │  │                   read_pos   write_pos          │   │
//! │  └─────────────────────────────────────────────────┘   │
//! │                                                         │
//! │  Line Index: [0, 15, 42, 78, 120, ...]                 │
//! │  (absolute positions of newlines)                       │
//! │                                                         │
//! └─────────────────────────────────────────────────────────┘
//! ```

use std::collections::VecDeque;

/// Default buffer capacity (64KB)
pub const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;

/// Default chunk size for emission (4KB)
pub const DEFAULT_CHUNK_SIZE: usize = 4 * 1024;

/// Ring buffer for streaming text with line indexing
///
/// Provides O(1) append and efficient line-based access for virtual scrolling.
pub struct StreamBuffer {
    /// Raw byte storage (circular)
    data: Vec<u8>,
    /// Write position in ring buffer
    write_pos: usize,
    /// Total bytes written (for absolute positioning)
    total_written: u64,
    /// Line start positions (absolute offsets)
    line_starts: VecDeque<u64>,
    /// Maximum capacity
    capacity: usize,
    /// Chunk size for emission
    chunk_size: usize,
}

impl StreamBuffer {
    /// Create a new stream buffer with default capacity
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_BUFFER_CAPACITY)
    }

    /// Create a stream buffer with custom capacity
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity],
            write_pos: 0,
            total_written: 0,
            line_starts: VecDeque::with_capacity(10000),
            capacity,
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }

    /// Set chunk size for emission
    pub fn set_chunk_size(&mut self, size: usize) {
        self.chunk_size = size.max(1).min(self.capacity);
    }

    /// Append bytes to the buffer
    ///
    /// Returns chunks ready for emission. Each chunk is at most `chunk_size` bytes.
    pub fn append(&mut self, bytes: &[u8]) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        let mut bytes_offset = 0; // Offset within input bytes

        while bytes_offset < bytes.len() {
            let chunk_size = (bytes.len() - bytes_offset).min(self.chunk_size);
            let chunk_bytes = &bytes[bytes_offset..bytes_offset + chunk_size];

            // Record the global offset before updating total_written
            let chunk_offset = self.total_written;

            // Index newlines (using absolute positions)
            for (i, &b) in chunk_bytes.iter().enumerate() {
                if b == b'\n' {
                    self.line_starts.push_back(chunk_offset + i as u64 + 1);
                }
            }

            // Write to ring buffer (handling wrap-around)
            let end = (self.write_pos + chunk_size) % self.capacity;
            if end > self.write_pos || chunk_size == 0 {
                self.data[self.write_pos..self.write_pos + chunk_size]
                    .copy_from_slice(chunk_bytes);
            } else {
                // Wrap around
                let first_part = self.capacity - self.write_pos;
                self.data[self.write_pos..].copy_from_slice(&chunk_bytes[..first_part]);
                self.data[..end].copy_from_slice(&chunk_bytes[first_part..]);
            }

            self.write_pos = end;
            self.total_written += chunk_size as u64;
            bytes_offset += chunk_size;

            chunks.push(StreamChunk {
                data: chunk_bytes.to_vec(),
                offset: chunk_offset,
            });
        }

        // Trim old line indices that are no longer in buffer
        let oldest_valid = self.total_written.saturating_sub(self.capacity as u64);
        while self
            .line_starts
            .front()
            .is_some_and(|&pos| pos < oldest_valid)
        {
            self.line_starts.pop_front();
        }

        chunks
    }

    /// Get total bytes written
    #[must_use]
    pub const fn total_written(&self) -> u64 {
        self.total_written
    }

    /// Get total line count
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Get capacity
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get bytes currently in buffer
    #[must_use]
    pub fn current_size(&self) -> usize {
        if self.total_written <= self.capacity as u64 {
            self.total_written as usize
        } else {
            self.capacity
        }
    }

    /// Read a range of bytes from the buffer
    ///
    /// Returns None if the range is not available (already overwritten).
    pub fn read_range(&self, start: u64, len: usize) -> Option<Vec<u8>> {
        let oldest_valid = self.total_written.saturating_sub(self.capacity as u64);

        // Check if range is still in buffer
        if start < oldest_valid || start >= self.total_written {
            return None;
        }

        let available = (self.total_written - start) as usize;
        let actual_len = len.min(available);

        // Calculate position in ring buffer
        let buffer_offset = if self.total_written <= self.capacity as u64 {
            start as usize
        } else {
            ((start - oldest_valid) as usize + self.write_pos) % self.capacity
        };

        let mut result = Vec::with_capacity(actual_len);

        // Handle wrap-around
        if buffer_offset + actual_len <= self.capacity {
            result.extend_from_slice(&self.data[buffer_offset..buffer_offset + actual_len]);
        } else {
            let first_part = self.capacity - buffer_offset;
            result.extend_from_slice(&self.data[buffer_offset..]);
            result.extend_from_slice(&self.data[..actual_len - first_part]);
        }

        Some(result)
    }

    /// Get visible lines for virtual scrolling
    ///
    /// Returns line content for the specified range of lines.
    /// Line 0 is the first line (from start of buffer or oldest available data).
    pub fn get_lines(&self, start_line: usize, count: usize) -> Vec<String> {
        let mut lines = Vec::with_capacity(count);

        // line_starts contains positions AFTER each newline (start of next line)
        // So line 0 starts at 0 (or oldest_valid), line 1 starts at line_starts[0], etc.
        let oldest_valid = self.total_written.saturating_sub(self.capacity as u64);

        for i in start_line..start_line + count {
            // Determine line start position
            let line_start = if i == 0 {
                oldest_valid
            } else if i <= self.line_starts.len() {
                self.line_starts[i - 1]
            } else {
                break;
            };

            // Determine line end position
            let line_end = if i < self.line_starts.len() {
                self.line_starts[i]
            } else {
                self.total_written
            };

            // Skip if this line is not in buffer
            if line_end <= oldest_valid {
                continue;
            }

            if let Some(bytes) = self.read_range(line_start, (line_end - line_start) as usize) {
                // Convert to string, trimming trailing newline
                let s = String::from_utf8_lossy(&bytes);
                lines.push(s.trim_end_matches('\n').to_string());
            }
        }

        lines
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.total_written = 0;
        self.line_starts.clear();
    }
}

impl Default for StreamBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// A chunk of stream data ready for emission
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Raw bytes
    pub data: Vec<u8>,
    /// Absolute offset in stream
    pub offset: u64,
}

impl StreamChunk {
    /// Create a new chunk
    #[must_use]
    pub fn new(data: Vec<u8>, offset: u64) -> Self {
        Self { data, offset }
    }

    /// Get data as string (lossy)
    #[must_use]
    pub fn as_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.data)
    }

    /// Get chunk size
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if chunk is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = StreamBuffer::new();
        assert_eq!(buffer.capacity(), DEFAULT_BUFFER_CAPACITY);
        assert_eq!(buffer.total_written(), 0);
        assert_eq!(buffer.line_count(), 0);
    }

    #[test]
    fn test_append_simple() {
        let mut buffer = StreamBuffer::with_capacity(1024);
        let chunks = buffer.append(b"Hello, World!\n");

        assert!(!chunks.is_empty());
        assert_eq!(buffer.total_written(), 14);
        assert_eq!(buffer.line_count(), 1);
    }

    #[test]
    fn test_append_multiple_lines() {
        let mut buffer = StreamBuffer::with_capacity(1024);
        buffer.append(b"Line 1\nLine 2\nLine 3\n");

        assert_eq!(buffer.line_count(), 3);
    }

    #[test]
    fn test_chunked_emission() {
        let mut buffer = StreamBuffer::with_capacity(1024);
        buffer.set_chunk_size(10);

        let data = b"0123456789ABCDEFGHIJ"; // 20 bytes
        let chunks = buffer.append(data);

        // Should emit 2 chunks of 10 bytes each
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].len(), 10);
        assert_eq!(chunks[1].len(), 10);
        // First chunk starts at offset 0
        assert_eq!(chunks[0].offset, 0);
        // Second chunk starts at offset 10
        assert_eq!(chunks[1].offset, 10);
        // Verify data content
        assert_eq!(&chunks[0].data, b"0123456789");
        assert_eq!(&chunks[1].data, b"ABCDEFGHIJ");
    }

    #[test]
    fn test_wrap_around() {
        let mut buffer = StreamBuffer::with_capacity(20);

        // Fill buffer
        buffer.append(b"12345678901234567890"); // 20 bytes

        // Write more to trigger wrap
        buffer.append(b"ABCDE"); // 5 more bytes

        assert_eq!(buffer.total_written(), 25);
        assert_eq!(buffer.current_size(), 20);
    }

    #[test]
    fn test_read_range() {
        let mut buffer = StreamBuffer::with_capacity(1024);
        buffer.append(b"Hello, World!");

        let result = buffer.read_range(0, 5);
        assert!(result.is_some());
        assert_eq!(result.as_deref(), Some(b"Hello".as_slice()));

        let result = buffer.read_range(7, 5);
        assert!(result.is_some());
        assert_eq!(result.as_deref(), Some(b"World".as_slice()));
    }

    #[test]
    fn test_get_lines() {
        let mut buffer = StreamBuffer::with_capacity(1024);
        buffer.append(b"Line 1\nLine 2\nLine 3\n");

        let lines = buffer.get_lines(0, 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
    }

    #[test]
    fn test_clear() {
        let mut buffer = StreamBuffer::with_capacity(1024);
        buffer.append(b"Some data\n");

        assert!(buffer.total_written() > 0);
        assert!(buffer.line_count() > 0);

        buffer.clear();

        assert_eq!(buffer.total_written(), 0);
        assert_eq!(buffer.line_count(), 0);
    }

    #[test]
    fn test_chunk_as_str_lossy() {
        let chunk = StreamChunk::new(b"Hello".to_vec(), 0);
        assert_eq!(chunk.as_str_lossy(), "Hello");

        // Test invalid UTF-8
        let chunk = StreamChunk::new(vec![0xFF, 0xFE], 0);
        let s = chunk.as_str_lossy();
        assert!(s.contains('\u{FFFD}')); // Replacement character
    }

    #[test]
    fn test_line_index_cleanup() {
        let mut buffer = StreamBuffer::with_capacity(100);

        // Add many lines to trigger cleanup
        for i in 0..50 {
            buffer.append(format!("Line {i}\n").as_bytes());
        }

        // Line indices for old content should be cleaned up
        // The exact count depends on buffer capacity
        assert!(buffer.line_count() < 50);
    }
}
