use std::{
    io::{ErrorKind, Read, Write},
    rc::Rc,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use crate::{
    error::{InstrumentError, Result},
    instrument::{
        info::{get_info, InstrumentInfo},
        Active, Info,
    },
};

use crate::interface::{Interface, NonBlock};

//create an Async version of the interface
pub struct AsyncStream {
    join: Option<JoinHandle<Result<Arc<dyn Interface + Send + Sync>>>>,
    write_to: Option<Sender<AsyncMessage>>,
    read_from: Rc<Receiver<Vec<u8>>>,
    stb_out: Rc<Receiver<StatusByte>>,
    buffer: Vec<u8>,
    nonblocking: bool,
    instrument_info: Option<InstrumentInfo>,
}

enum AsyncMessage {
    Message(Vec<u8>),
    Stb,
    End,
}

impl AsyncStream {
    fn write_stb_request(&self) -> Result<()> {
        self.write_to.as_ref().map_or_else(
            || Ok(()),
            |s| match s.send(AsyncMessage::Stb) {
                Ok(()) => Ok(()),
                Err(e) => Err(InstrumentError::Other(format!(
                    "Unable to request STB: {e}"
                ))),
            },
        )
    }

    fn read_stb(&self) -> Result<StatusByte> {
        match self.stb_out.recv() {
            Ok(s) => Ok(s),
            Err(e) => Err(crate::InstrumentError::Other(format!(
                "unable to get STB from device: {e}"
            ))),
        }
    }

    fn join_thread(&mut self) -> Result<Arc<dyn Interface + Send + Sync>> {
        self.drop_write_channel()?;
        let socket = match self.join.take() {
            Some(join) => match join.join() {
                Ok(Ok(socket)) => socket,
                _ => {
                    return Err(InstrumentError::ConnectionError {
                        details: "unable to retrieve synchronous stream".to_string(),
                    });
                }
            },
            None => {
                return Err(InstrumentError::ConnectionError {
                    details: "unable to close the asynchronous connection, could not retrieve synchronous stream".to_string(),
                });
            }
        };

        Ok(socket)
    }

    fn drop_write_channel(&mut self) -> Result<()> {
        if let Some(send) = self.write_to.take() {
            match send.send(AsyncMessage::End) {
                Ok(()) => {}
                Err(_) => {
                    return Err(InstrumentError::IoError {
                        source: (std::io::Error::new(
                            ErrorKind::NotConnected,
                            "attempted to write asynchronously to socket, but it was not connected"
                                .to_string(),
                        )),
                    });
                }
            }
        }
        Ok(())
    }
}

impl TryFrom<Arc<dyn Interface + Send + Sync>> for AsyncStream {
    type Error = InstrumentError;

    fn try_from(
        mut socket: Arc<dyn Interface + Send + Sync>,
    ) -> std::result::Result<Self, Self::Error> {
        let (write_to, read_into) = mpsc::channel();
        let (write_out, read_from) = mpsc::channel();
        let (stb_from, stb_out) = mpsc::channel();
        let builder =
            std::thread::Builder::new().name("Instrument Communication Thread".to_string());
        let inst = Arc::get_mut(&mut socket).unwrap().info()?;
        //TODO: Populate name with instrument information
        // get INstrumentInfo by call get_info of interface
        let join = builder.spawn(move || -> Result<Arc<dyn Interface + Send + Sync>> {
            Arc::get_mut(&mut socket).unwrap().set_nonblocking(true)?;
            let read_into: Receiver<AsyncMessage> = read_into;
            let write_out: Sender<Vec<u8>> = write_out;
            let stb_from: Sender<StatusByte> = stb_from;

            'rw_loop: loop {
                // see if the application has anything to send to the instrument.
                std::thread::sleep(Duration::from_millis(1));
                match read_into.try_recv() {
                    // It does, so send it
                    Ok(AsyncMessage::Message(msg)) => {
                        let chunk_size = 1024;
                        let mut start = 0;
                        while start < msg.len() {
                            let end = std::cmp::min(
                                start.checked_add(chunk_size).unwrap_or(usize::MAX),
                                msg.len(),
                            );
                            let chunk = &msg[start..end];
                            let mut bytes_sent = 0;
                            loop {
                                match Arc::get_mut(&mut socket)
                                    .unwrap()
                                    // Do NOT add a newline here. It is added elsewhere.
                                    .write(&chunk[bytes_sent..])
                                {
                                    Ok(0) => {
                                        // All data has been sent
                                        break;
                                    }
                                    Ok(n) => {
                                        // Successfully sent some data
                                        bytes_sent =
                                            bytes_sent.checked_add(n).unwrap_or(usize::MAX);
                                    }
                                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                        // Non-blocking write would block
                                        // You can do other work here, like processing other tasks or sleeping
                                        std::thread::sleep(Duration::from_millis(1));
                                    }
                                    Err(e) => {
                                        // There was an Error sending to the instrument.
                                        // clean up and get out.
                                        return Err(e.into());
                                    }
                                }
                            }
                            start = start.checked_add(chunk_size).unwrap_or(msg.len());
                        }
                    }

                    Ok(AsyncMessage::End) | Err(mpsc::TryRecvError::Disconnected) => {
                        break 'rw_loop;
                    }

                    Ok(AsyncMessage::Stb) => {
                        let socket = Arc::get_mut(&mut socket).unwrap();
                        socket.write_all(b"*STB?\n")?;
                        std::thread::sleep(Duration::from_millis(1));
                        let mut stb_buf = vec![0u8; 4];
                        let _ = socket.read(&mut stb_buf)?;
                        let _ = stb_from.send(StatusByte::from(stb_buf.as_slice()));
                    }

                    Err(mpsc::TryRecvError::Empty) => {}
                }
                let buf = &mut [0u8; 512];
                if let Ok(size) = Arc::get_mut(&mut socket).unwrap().read(buf) {
                    let buf = &buf[..size];
                    // This `send()` sends this to a receiver that will be activated next time a self.read() is called.
                    if size > 0 && write_out.send(buf.to_vec()).is_err() {
                        return Err(std::io::Error::new(
                            ErrorKind::ConnectionReset,
                            "attempted to send message from device, but client was closed"
                                .to_string(),
                        )
                        .into());
                    }
                }
            }
            Ok(socket)
        })?;

        Ok(Self {
            join: Some(join),
            write_to: Some(write_to),
            read_from: Rc::new(read_from),
            stb_out: Rc::new(stb_out),
            buffer: Vec::new(),
            nonblocking: true,
            instrument_info: Some(inst),
        })
    }
}

impl Read for AsyncStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // define an error since all of the returned errors are for the same reason.
        let error: std::io::Error = std::io::Error::new(
            ErrorKind::NotConnected,
            "attempted to read asynchronously from socket, but it was not connected".to_string(),
        );

        let resp = if self.nonblocking {
            match self.read_from.try_recv() {
                Ok(resp) => resp,
                Err(e) => match e {
                    TryRecvError::Empty => Vec::default(),
                    TryRecvError::Disconnected => {
                        return Err(error);
                    }
                },
            }
        } else {
            if !self.buffer.is_empty() {
                let read_size = self.buffer.take(buf.len() as u64).read(buf)?;
                self.buffer = self.buffer[read_size..].into();
                return Ok(read_size);
            }
            match self.read_from.recv() {
                Ok(resp) => resp,
                Err(_) => {
                    return Err(error);
                }
            }
        };

        let _ = self.buffer.write(&resp);
        let read_size = self.buffer.take(buf.len() as u64).read(buf)?;
        self.buffer = self.buffer[read_size..].into();

        Ok(read_size)
    }
}

impl Write for AsyncStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self.write_to {
            Some(ref mut send) => match send.send(AsyncMessage::Message(Vec::from(buf))) {
                Ok(()) => {}
                Err(_) => {
                    return Err(std::io::Error::new(
                        ErrorKind::NotConnected,
                        "attempted to write asynchronously to socket, but it was not connected"
                            .to_string(),
                    ));
                }
            },
            None => {
                return Err(std::io::Error::new(
                    ErrorKind::NotConnected,
                    "asynchronous connection was not found".to_string(),
                ));
            }
        };

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl NonBlock for AsyncStream {
    fn set_nonblocking(&mut self, nonblocking: bool) -> Result<()> {
        self.nonblocking = nonblocking;
        Ok(())
    }
}

impl Info for AsyncStream {
    fn info(&mut self) -> Result<InstrumentInfo> {
        if let Some(inst_info) = self.instrument_info.clone() {
            return Ok(inst_info);
        }

        get_info(self)
    }
}

impl Drop for AsyncStream {
    fn drop(&mut self) {
        let _ = self.join_thread();
    }
}

impl TryFrom<AsyncStream> for Arc<dyn Interface + Send + Sync> {
    type Error = InstrumentError;

    fn try_from(async_stream: AsyncStream) -> std::result::Result<Self, Self::Error> {
        let mut async_stream = async_stream;
        let socket = async_stream.join_thread()?;
        Ok(socket)
    }
}

impl Active for AsyncStream {
    fn get_status(&mut self) -> Result<StatusByte> {
        self.write_stb_request()?;
        self.read_stb()
    }
}

impl Interface for AsyncStream {}

const fn bit_check(byte: u8, bit: usize) -> bool {
    let mask = 1 << bit;
    ((byte & mask) >> bit) > 0
}

/// Represents that Status Byte of an instrument
#[derive(Debug, Default, Clone, Copy)]
pub struct StatusByte {
    byte: u8,
}

impl StatusByte {
    /// Create a status byte from a literal [`u8`]
    ///
    /// NOTE: The u8 here must NOT be the string representation of the status
    /// byte number.
    ///
    /// # Panics
    /// In debug builds, this will panic if the given byte has the unused bits
    /// (1 and 6), meaning that the passed byte was likely an accidental string
    /// representation of the status byte.
    #[must_use]
    pub const fn new(byte: u8) -> Self {
        debug_assert!(!bit_check(byte, 1) && !bit_check(byte, 6));
        Self { byte }
    }

    /// If `true`, an enabled event in the Measurement Event Register has
    /// occurred
    #[must_use]
    pub const fn msb(&self) -> bool {
        bit_check(self.byte, 0)
    }

    /// If `true`, an error or status message is present in the Error Queue
    #[must_use]
    pub const fn eav(&self) -> bool {
        bit_check(self.byte, 2)
    }

    /// If `true`, an enabled even in the Questionable Status Register has
    /// occurred
    #[must_use]
    pub const fn qsb(&self) -> bool {
        bit_check(self.byte, 3)
    }

    /// If `true`, a response message is present in the Output Queue
    #[must_use]
    pub const fn mav(&self) -> bool {
        bit_check(self.byte, 4)
    }

    /// If `true`, an enabled event in the Standard Event Status Register has
    /// occurred
    #[must_use]
    pub const fn esb(&self) -> bool {
        bit_check(self.byte, 5)
    }
    /// If `true`, an enabled event in the Operation Status Register has
    /// occurred
    #[must_use]
    pub const fn osb(&self) -> bool {
        bit_check(self.byte, 5)
    }
}

impl From<&[u8]> for StatusByte {
    fn from(value: &[u8]) -> Self {
        Self::new(
            String::from_utf8_lossy(value)
                .trim()
                .parse::<u8>()
                .unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod unit {
    use super::{bit_check, StatusByte};

    #[test]
    fn bit_check_processes_bytes() {
        let tests = [
            (
                0b0000_0000,
                [false, false, false, false, false, false, false, false],
            ),
            (
                0b0000_0001,
                [true, false, false, false, false, false, false, false],
            ),
            (
                0b0000_0010,
                [false, true, false, false, false, false, false, false],
            ),
            (
                0b0000_0100,
                [false, false, true, false, false, false, false, false],
            ),
            (
                0b0000_1000,
                [false, false, false, true, false, false, false, false],
            ),
            (
                0b0001_0000,
                [false, false, false, false, true, false, false, false],
            ),
            (
                0b0010_0000,
                [false, false, false, false, false, true, false, false],
            ),
            (
                0b0100_0000,
                [false, false, false, false, false, false, true, false],
            ),
            (
                0b1000_0000,
                [false, false, false, false, false, false, false, true],
            ),
        ];

        for (byte, expected) in tests {
            for (i, e) in expected.iter().enumerate() {
                assert_eq!(bit_check(byte, i), *e);
            }
        }
    }

    #[test]
    fn status_byte_from_string() {
        for a in (0x00..0xFF).filter(|&x| !(bit_check(x, 1) || bit_check(x, 6))) {
            assert_eq!(StatusByte::from(format!("{a}\n").as_bytes()).byte, a);
        }
    }
}
