#![feature(assert_matches, rustdoc_missing_doc_code_examples)]
#![doc(html_logo_url = "../../../ki-comms_doc_icon.png")]

//! The TSP Instrument crate defines the necessary components to enable communication
//! with an instrument at various levels of abstraction. The aim of the library is to
//! interact correctly, efficiently and reliably with Keithley instruments via multiple
//! connection interfaces. Right now LAN and USBTMC are implemented, and VISA is
//! planned

//pub mod connect;
pub mod error;
pub mod instrument;
pub mod interface;
pub mod model;

#[cfg(test)]
pub(crate) mod test_util;

pub use error::InstrumentError;
pub use instrument::firmware::Flash;
pub use interface::{connection_addr::ConnectionAddr, usbtmc, Interface};
pub use model::{ki2600, tti, versatest};

pub mod protocol;
pub use protocol::is_visa_installed;

pub mod new {
    use std::{
        error::Error,
        fmt::Display,
        io::{Read, Write},
        str::FromStr,
        time::Duration,
    };

    use visa_rs::enums::assert::AssertTrigPro;

    use std::net::TcpStream;

    use crate::{instrument::info::InstrumentInfo, InstrumentError};

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

    pub trait Info {
        type Error: Display + Error;

        /// Get the information for the instrument.
        ///
        /// # Errors
        /// The errors returned must be of, or convertible to the type `Self::Error`.
        fn info(&mut self) -> core::result::Result<InstrumentInfo, Self::Error>;
    }

    pub enum Protocol {
        RawSocket(TcpStream),
        Visa {
            instr: visa_rs::Instrument,
            rm: visa_rs::DefaultRM,
        },
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

    #[allow(non_camel_case_types)]
    pub enum Model {
        //26xx
        _2601,
        _2602,
        _2611,
        _2612,
        _2635,
        _2636,
        _2601A,
        _2602A,
        _2611A,
        _2612A,
        _2635A,
        _2636A,
        _2651A,
        _2657A,
        _2601B,
        _2601B_PULSE,
        _2602B,
        _2606B,
        _2611B,
        _2612B,
        _2635B,
        _2636B,
        _2604B,
        _2614B,
        _2634B,
        _2601B_L,
        _2602B_L,
        _2611B_L,
        _2612B_L,
        _2635B_L,
        _2636B_L,
        _2604B_L,
        _2614B_L,
        _2634B_L,

        // 3706 or 70xB
        _3706,
        _3706_SNFP,
        _3706_S,
        _3706_NFP,
        _3706A,
        _3706A_SNFP,
        _3706A_S,
        _3706A_NFP,
        _707B,
        _708B,

        // TTI
        _2450,
        _2470,
        _DMM7510,
        _2460,
        _2461,
        _2461_SYS,
        DMM7512,
        DMM6500,
        DAQ6510,

        // Modular Platform
        Mp5103,

        // Anything else
        Other,
    }

    impl FromStr for Model {
        type Err = InstrumentError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            Ok(match s {
                //26xx
                "2601" => Self::_2601,
                "2602" => Self::_2602,
                "2611" => Self::_2611,
                "2612" => Self::_2612,
                "2635" => Self::_2635,
                "2636" => Self::_2636,
                "2601A" => Self::_2601A,
                "2602A" => Self::_2602A,
                "2611A" => Self::_2611A,
                "2612A" => Self::_2612A,
                "2635A" => Self::_2635A,
                "2636A" => Self::_2636A,
                "2651A" => Self::_2651A,
                "2657A" => Self::_2657A,
                "2601B" => Self::_2601B,
                "2601B-PULSE" => Self::_2601B_PULSE,
                "2602B" => Self::_2602B,
                "2606B" => Self::_2606B,
                "2611B" => Self::_2611B,
                "2612B" => Self::_2612B,
                "2635B" => Self::_2635B,
                "2636B" => Self::_2636B,
                "2604B" => Self::_2604B,
                "2614B" => Self::_2614B,
                "2634B" => Self::_2634B,
                "2601B-L" => Self::_2601B_L,
                "2602B-L" => Self::_2602B_L,
                "2611B-L" => Self::_2611B_L,
                "2612B-L" => Self::_2612B_L,
                "2635B-L" => Self::_2635B_L,
                "2636B-L" => Self::_2636B_L,
                "2604B-L" => Self::_2604B_L,
                "2614B-L" => Self::_2614B_L,
                "2634B-L" => Self::_2634B_L,

                // 3706 or 70xB
                "3706" => Self::_3706,
                "3706-SNFP" => Self::_3706_SNFP,
                "3706-S" => Self::_3706_S,
                "3706-NFP" => Self::_3706_NFP,
                "3706A" => Self::_3706A,
                "3706A-SNFP" => Self::_3706A_SNFP,
                "3706A-S" => Self::_3706A_S,
                "3706A-NFP" => Self::_3706A_NFP,
                "707B" => Self::_707B,
                "708B" => Self::_708B,

                // TTI
                "2450" => Self::_2450,
                "2470" => Self::_2470,
                "DMM7510-" => Self::_DMM7510,
                "2460" => Self::_2460,
                "2461" => Self::_2461,
                "2461-SYS" => Self::_2461_SYS,
                "DMM7512" => Self::DMM7512,
                "DMM6500" => Self::DMM6500,
                "DAQ6510" => Self::DAQ6510,

                // Modular Platform
                "MP5103" | "VERSATEST-300" | "VERSATEST-600" | "TSPop" | "TSP" => Self::Mp5103,

                //Other
                _ => Self::Other,
            })
        }
    }

    #[allow(dead_code)]
    pub struct Instrument {
        protocol: Protocol,
        model: Model,
    }
}
