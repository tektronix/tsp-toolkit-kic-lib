use std::{
    error::Error,
    fmt::Display,
    io::{Read, Write},
};

use visa_rs::{
    enums::assert::AssertTrigPro, flags::AccessMode, AsResourceManager, DefaultRM, VisaString,
    TIMEOUT_IMMEDIATE,
};

use crate::{
    error::Result,
    instrument::{info::get_info, Info},
    InstrumentError, Interface,
};

pub enum Protocol {
    Raw(Box<dyn Interface>),
    Visa(visa_rs::Instrument),
}

pub enum Stb {
    Stb(u16),
    NotSupported,
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
            Self::Visa(v) => v.read(buf),
        }
    }
}

impl Write for Protocol {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Raw(r) => r.write(buf),
            Self::Visa(v) => v.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Raw(r) => r.flush(),
            Self::Visa(v) => v.flush(),
        }
    }
}

impl Info for Protocol {
    fn info(&mut self) -> crate::error::Result<crate::instrument::info::InstrumentInfo> {
        match self {
            Self::Raw(r) => r.info(),
            Self::Visa(v) => get_info(v),
        }
    }
}

impl Clear for Protocol {
    type Error = InstrumentError;
    fn clear(&mut self) -> core::result::Result<(), Self::Error> {
        match self {
            Self::Raw(r) => r.write_all(b"*CLS")?,
            Self::Visa(v) => v.clear()?,
        };

        Ok(())
    }
}

impl ReadStb for Protocol {
    type Error = InstrumentError;
    fn read_stb(&mut self) -> core::result::Result<Stb, Self::Error> {
        match self {
            Self::Raw(_) => Ok(Stb::NotSupported),
            Self::Visa(v) => Ok(Stb::Stb(v.read_stb()?)),
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
            Self::Visa(v) => {
                v.assert_trigger(AssertTrigPro::TrigProtDefault)?;
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
    pub fn try_from_visa(visa_string: String) -> Result<Self> {
        let rm = DefaultRM::new()?;
        let Some(resource_string) = VisaString::from_string(visa_string.clone()) else {
            return Err(InstrumentError::AddressParsingError {
                unparsable_string: visa_string,
            });
        };
        let rsc = rm.find_res(&resource_string)?;
        let instr: visa_rs::Instrument = rm.open(&rsc, AccessMode::NO_LOCK, TIMEOUT_IMMEDIATE)?;

        Ok(Self::Visa(instr))
    }
}

impl From<Box<dyn Interface>> for Protocol {
    fn from(value: Box<dyn Interface>) -> Self {
        Self::Raw(value)
    }
}
