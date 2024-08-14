use std::{
    error::Error,
    fmt::Display,
    io::{ErrorKind, Read, Write},
};

use tracing::{trace, warn};
use visa_rs::{
    enums::assert::AssertTrigPro, flags::AccessMode, AsResourceManager, DefaultRM, VisaString,
    TIMEOUT_INFINITE,
};

use crate::{
    error::Result,
    instrument::{info::get_info, Info},
    InstrumentError, Interface,
};

pub enum Protocol {
    Raw(Box<dyn Interface>),
    Visa {
        instr: visa_rs::Instrument,
        rm: DefaultRM,
    },
}

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
    pub fn mav(&self) -> Result<bool> {
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
    pub fn esr(&self) -> Result<bool> {
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
    pub fn srq(&self) -> Result<bool> {
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
    fn read_stb(&mut self) -> core::result::Result<Stb, Self::Error> {
        Ok(Stb::NotSupported)
    }
}

pub trait Clear {
    type Error: Display + Error;
    /// # Errors
    /// The errors returned must be of, or convertible to the type `Self::Error`.
    fn clear(&mut self) -> core::result::Result<(), Self::Error>;
}

pub trait Trigger {
    type Error: Display + Error;

    /// # Errors
    /// The errors returned must be of, or convertible to the type `Self::Error`.
    fn trigger(&mut self) -> core::result::Result<(), Self::Error>;
}

impl Read for Protocol {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Raw(r) => r.read(buf),
            Self::Visa { instr, .. } => {
                let stb = Stb::Stb(instr.read_stb().map_err(|e| {
                    std::io::Error::new(ErrorKind::Other, format!("error reading STB: {e}"))
                })?);

                if matches!(stb.mav(), Ok(false)) {
                    return Ok(0);
                }
                instr.read(buf)
            }
        }
    }
}

impl Write for Protocol {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Raw(r) => r.write(buf),
            Self::Visa { instr, .. } => instr.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Raw(r) => r.flush(),
            Self::Visa { instr, .. } => instr.flush(),
        }
    }
}

impl Info for Protocol {
    fn info(&mut self) -> crate::error::Result<crate::instrument::info::InstrumentInfo> {
        match self {
            Self::Raw(r) => r.info(),
            Self::Visa { instr, .. } => get_info(instr),
        }
    }
}

impl Clear for Protocol {
    type Error = InstrumentError;
    fn clear(&mut self) -> core::result::Result<(), Self::Error> {
        match self {
            Self::Raw(r) => r.write_all(b"*CLS")?,
            Self::Visa { instr, .. } => instr.clear()?,
        };

        Ok(())
    }
}

impl ReadStb for Protocol {
    type Error = InstrumentError;
    fn read_stb(&mut self) -> core::result::Result<Stb, Self::Error> {
        match self {
            Self::Raw(_) => Ok(Stb::NotSupported),
            Self::Visa { instr, .. } => Ok(Stb::Stb(instr.read_stb()?)),
        }
    }
}

impl Trigger for Protocol {
    type Error = InstrumentError;
    fn trigger(&mut self) -> core::result::Result<(), Self::Error> {
        match self {
            Self::Raw(r) => {
                r.write_all(b"*TRG\n")?;
            }
            Self::Visa { instr, .. } => {
                instr.assert_trigger(AssertTrigPro::TrigProtDefault)?;
            }
        }
        Ok(())
    }
}

impl Protocol {
    /// Try to convert a visa string to a Protocol.
    /// Note that (for now) this will always return a Visa instrument.
    ///
    /// # Errors
    /// Errors may occur from the system Visa drivers.
    #[tracing::instrument]
    pub fn try_from_visa(visa_string: String) -> Result<Self> {
        trace!("Getting VISA Resource Manager");
        let rm = DefaultRM::new()?;
        trace!("Converting given resource string to VisaString");
        let Some(resource_string) = VisaString::from_string(visa_string.clone()) else {
            return Err(InstrumentError::AddressParsingError {
                unparsable_string: visa_string,
            });
        };
        trace!("Finding resource");
        let mut rsc = rm.find_res_list(&resource_string)?;
        trace!("Resource List: {rsc:?}");
        let Some(rsc) = rsc.find_next()? else {
            warn!("No resource found");
            return Err(InstrumentError::ConnectionError {
                details: "unable to find requested resource".to_string(),
            });
        };
        trace!("Resource: {rsc:?}");
        trace!("Opening resource");
        let instr: visa_rs::Instrument = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_INFINITE)?;
        trace!("Opened instrument: {instr:?}");

        Ok(Self::Visa { instr, rm })
    }
}

impl From<Box<dyn Interface>> for Protocol {
    fn from(value: Box<dyn Interface>) -> Self {
        Self::Raw(value)
    }
}
