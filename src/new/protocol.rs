use std::{
    error::Error,
    fmt::Display,
    io::{ErrorKind, Read, Write},
    net::TcpStream,
    time::Duration,
};

use tracing::{debug, trace};
use visa_rs::{enums::assert::AssertTrigPro, AsResourceManager};

use crate::{ConnectionAddr, InstrumentError};

use super::{Clear, Info, InstrumentInfo, Trigger};

pub enum Stb {
    Stb(u16),
    NotSupported,
}

impl Stb {
    const fn is_bit_set(stb: u16, bit: u8) -> bool {
        if bit > 15 {
            return false;
        }

        ((stb >> bit) & 0x0001) == 1
    }

    /// Check to see if the MAV bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn mav(&self) -> core::result::Result<bool, InstrumentError> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 4)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the ESR bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn esr(&self) -> core::result::Result<bool, InstrumentError> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 5)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }

    /// Check to see if the SRQ bit is set
    ///
    /// # Errors
    /// An error is returned if `read_stb` is not supported.
    pub fn srq(&self) -> core::result::Result<bool, InstrumentError> {
        match self {
            Self::Stb(s) => Ok(Self::is_bit_set(*s, 6)),
            Self::NotSupported => Err(InstrumentError::Other(
                "read_stb() not supported".to_string(),
            )),
        }
    }
}

pub trait ReadStb {
    type Error: Display + Error;
    /// # Errors
    /// The errors returned must be of, or convertible to the type `Self::Error`.
    fn read_stb(&mut self) -> core::result::Result<Stb, Self::Error>;
}
pub enum Protocol {
    RawSocket(TcpStream),
    Visa {
        instr: visa_rs::Instrument,
        rm: visa_rs::DefaultRM,
    },
}

impl TryFrom<TcpStream> for Protocol {
    type Error = InstrumentError;
    fn try_from(value: TcpStream) -> core::result::Result<Self, Self::Error> {
        //value.set_nonblocking(true)?;
        Ok(Self::RawSocket(value))
    }
}

impl TryFrom<ConnectionAddr> for Protocol {
    type Error = InstrumentError;

    fn try_from(value: ConnectionAddr) -> Result<Self, Self::Error> {
        match value {
            ConnectionAddr::Lan(socket_addr) => {
                let stream = TcpStream::connect(socket_addr)?;
                stream.try_into()
            }
            ConnectionAddr::Visa(visa_string) => {
                let rm = visa_rs::DefaultRM::new()?;
                let instr = rm.open(
                    &visa_string,
                    visa_rs::flags::AccessMode::NO_LOCK,
                    Duration::from_secs(5),
                )?;
                Ok(Self::Visa { instr, rm })
            }
            ConnectionAddr::Unknown => Err(InstrumentError::ConnectionError {
                details: "connection address unknown".to_string(),
            }),
        }
    }
}

impl Write for Protocol {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::RawSocket(tcp_stream) => tcp_stream.write(buf),
            Self::Visa { instr, .. } => instr.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::RawSocket(tcp_stream) => tcp_stream.flush(),
            Self::Visa { instr, .. } => instr.flush(),
        }
    }
}

impl Read for Protocol {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::RawSocket(tcp_stream) => tcp_stream.read(buf),
            Self::Visa { instr, .. } => instr.read(buf),
        }
    }
}

impl ReadStb for Protocol {
    type Error = InstrumentError;

    fn read_stb(&mut self) -> core::result::Result<Stb, Self::Error> {
        Ok(match self {
            Self::RawSocket(s) => {
                let buf = &mut [0u8; 5][..];
                let x = match s.peek(buf) {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(e.into());
                    }
                };
                if x > 0 {
                    // set MAV
                    Stb::Stb(0x0010)
                } else {
                    // don't set MAV
                    Stb::Stb(0x0000)
                }
            }
            Self::Visa { instr, .. } => Stb::Stb(instr.read_stb()?),
        })
    }
}

impl Clear for Protocol {
    type Error = InstrumentError;

    fn clear(&mut self) -> core::result::Result<(), Self::Error> {
        match self {
            Self::RawSocket(tcp_stream) => tcp_stream.write_all(b"*CLS\n")?,
            Self::Visa { instr, .. } => instr.clear()?,
        };
        Ok(())
    }
}

impl Trigger for Protocol {
    type Error = InstrumentError;

    fn trigger(&mut self) -> core::result::Result<(), Self::Error> {
        match self {
            Self::RawSocket(tcp_stream) => tcp_stream.write_all(b"*TRG\n")?,
            Self::Visa { instr, .. } => {
                instr.assert_trigger(AssertTrigPro::TrigProtDefault)?;
            }
        }
        Ok(())
    }
}

impl Info for Protocol {
    type Error = InstrumentError;

    #[tracing::instrument(skip(self))]
    fn info(&mut self) -> core::result::Result<InstrumentInfo, Self::Error> {
        debug!("Writing *IDN?");
        self.write_all(b"*IDN?\n")?;
        let mut i = 0u8;
        let info: Option<InstrumentInfo> = loop {
            std::thread::sleep(Duration::from_millis(100));
            i = i.saturating_add(1);
            if i > 100 {
                break None;
            }
            if !self.read_stb()?.mav()? {
                trace!("attempt {i}: MAV false");
                continue;
            }
            trace!("attempt {i}: MAV true");

            let mut buf = vec![0u8; 256];

            match self.read(&mut buf) {
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.into()),
            }
            let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
            let buf = &buf[..first_null];
            trace!("Got {} from *IDN?", String::from_utf8_lossy(buf));
            if let Ok(i) = buf.try_into() {
                break Some(i);
            }

        };
        info.ok_or(InstrumentError::InformationRetrievalError {
            details: "unable to read instrument info".to_string(),
        })
    }
}
