use std::net::TcpStream;
use std::time::Duration;

use oya_ipc::{GuestMessage, HostMessage, IpcTransport, TransportError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IpcError {
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Transport error: {0}")]
    Transport(String),
}

pub struct IpcClient {
    transport: IpcTransport<TcpStream, TcpStream>,
}

impl IpcClient {
    /// Connect to the IPC server.
    ///
    /// # Errors
    /// Returns an error when the connection fails.
    pub fn connect(address: &str) -> Result<Self, IpcError> {
        let stream = TcpStream::connect(address).map_err(|err| IpcError::Io(err.to_string()))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|err| IpcError::Io(err.to_string()))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|err| IpcError::Io(err.to_string()))?;

        let reader = stream
            .try_clone()
            .map_err(|err| IpcError::Io(err.to_string()))?;
        let transport = IpcTransport::new(reader, stream);
        Ok(Self { transport })
    }

    /// Send a request and wait for a response.
    ///
    /// # Errors
    /// Returns an error when sending or receiving fails.
    pub fn request(&mut self, message: GuestMessage) -> Result<HostMessage, IpcError> {
        self.transport.send(&message).map_err(map_transport_error)?;
        self.transport
            .recv::<HostMessage>()
            .map_err(map_transport_error)
    }
}

fn map_transport_error(error: TransportError) -> IpcError {
    IpcError::Transport(error.to_string())
}
