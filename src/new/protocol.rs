use std::{
    error::Error,
    fmt::Display,
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use visa_rs::enums::assert::AssertTrigPro;

use crate::InstrumentError;

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
            Self::RawSocket(_) => Stb::NotSupported,
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

    fn info(&mut self) -> core::result::Result<InstrumentInfo, Self::Error> {
        fn get_info(
            instr: &mut (impl Read + Write),
        ) -> core::result::Result<InstrumentInfo, InstrumentError> {
            instr.write_all(b"*IDN?\n")?;
            let mut info: Option<InstrumentInfo> = None;
            for _ in 0..100 {
                std::thread::sleep(Duration::from_millis(100));

                let mut buf = vec![0u8; 100];
                let _ = instr.read(&mut buf)?;
                let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
                let buf = &buf[..first_null];
                if let Ok(i) = buf.try_into() {
                    info = Some(i);
                    break;
                }
            }
            info.ok_or(InstrumentError::InformationRetrievalError {
                details: "unable to read instrument info".to_string(),
            })
        }

        match self {
            Self::RawSocket(instr) => get_info(instr),
            Self::Visa { instr, .. } => get_info(instr),
        }
    }
}
