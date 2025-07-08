use std::{fmt::Display, str::FromStr};

use tracing::{instrument, trace};

use crate::{
    instrument::{authenticate::Authentication, Instrument},
    interface::connection_addr::ConnectionInfo,
    protocol::Protocol,
    InstrumentError,
};

pub mod ki2600;
pub mod ki3700;
pub mod tti;
pub mod versatest;

#[must_use]
pub fn is_supported(model: impl AsRef<str>) -> bool {
    self::ki2600::Instrument::model_is(&model)
        || self::ki3700::Instrument::model_is(&model)
        || self::tti::Instrument::model_is(&model)
        || self::versatest::Instrument::model_is(&model)
}

/// Connect to an instrument given the instrument's connection information and authentication
/// info.
///
/// # Errors
/// Errors may occur when getting the model (for LAN-based connections, this would
/// likely be a [`reqwest`] error from trying to fetch the LXI Identification page).
/// IO errors or parsing errors are possible. There could be errors in establishing the
/// connection as well.
#[instrument(skip(conn, auth))]
pub fn connect_to(
    conn: &ConnectionInfo,
    auth: Authentication,
) -> Result<Box<dyn Instrument>, InstrumentError> {
    trace!("Connecting to {conn}");
    let model: Model = conn.get_model()?;

    Ok(if model.is_2600() {
        Box::new(ki2600::Instrument::connect(conn, auth)?)
    } else if model.is_3700_70x() {
        Box::new(ki3700::Instrument::connect(conn, auth)?)
    } else if model.is_tti() {
        Box::new(tti::Instrument::connect(conn, auth)?)
    } else if model.is_mp() {
        Box::new(versatest::Instrument::connect(conn, auth)?)
    } else {
        trace!("Unable to determine instrument model, defaulting to MP5000 series connection procedure.");
        Box::new(versatest::Instrument::connect(conn, auth)?)
    })
}

/// Connect to an instrument given the instrument's connection information and authentication
/// info.
///
/// # Errors
/// Errors may occur when getting the model (for LAN-based connections, this would
/// likely be a [`reqwest`] error from trying to fetch the LXI Identification page).
/// IO errors or parsing errors are possible. There could be errors in establishing the
/// connection as well.
pub fn connect_protocol(
    conn: &ConnectionInfo,
    proto: Protocol,
    auth: Authentication,
) -> Result<Box<dyn Instrument>, InstrumentError> {
    let model: Model = conn.get_model()?;

    Ok(if model.is_2600() {
        Box::new(ki2600::Instrument::new(proto, auth))
    } else if model.is_3700_70x() {
        Box::new(ki3700::Instrument::new(proto, auth))
    } else if model.is_tti() {
        Box::new(tti::Instrument::new(proto, auth))
    } else if model.is_mp() {
        Box::new(versatest::Instrument::new(proto, auth))
    } else {
        trace!("Unable to determine instrument model, defaulting to MP5000 series connection procedure.");
        Box::new(versatest::Instrument::new(proto, auth))
    })
}

//impl TryFrom<Protocol> for Box<dyn Instrument> {
//    type Error = InstrumentError;
//
//    fn try_from(mut proto: Protocol) -> Result<Self, Self::Error> {
//        let info = proto.info()?;
//        let auth = Box::new(Authenticate {});
//        if tti::Instrument::is(&info) {
//            let mut ins = Box::new(tti::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki2600::Instrument::is(&info) {
//            let mut ins = Box::new(ki2600::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki3700::Instrument::is(&info) {
//            let mut ins = Box::new(ki3700::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if versatest::Instrument::is(&info) {
//            let mut ins = Box::new(versatest::Instrument::new(proto, auth));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else {
//            Err(InstrumentError::InstrumentError {
//                error: "unable to determine instrument type".to_string(),
//            })
//        }
//    }
//}
//
//impl TryFrom<Box<dyn Interface>> for Box<dyn Instrument> {
//    type Error = InstrumentError;
//
//    fn try_from(mut interface: Box<dyn Interface>) -> std::result::Result<Self, Self::Error> {
//        let info = interface.as_mut().info()?;
//        let auth = Box::new(Authenticate {});
//        if tti::Instrument::is(&info) {
//            let mut ins = Box::new(tti::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki2600::Instrument::is(&info) {
//            let mut ins = Box::new(ki2600::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if ki3700::Instrument::is(&info) {
//            let mut ins = Box::new(ki3700::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else if versatest::Instrument::is(&info) {
//            let mut ins = Box::new(versatest::Instrument::new(
//                protocol::Protocol::Raw(interface),
//                auth,
//            ));
//            ins.as_mut().add_info(info);
//            Ok(ins)
//        } else {
//            Err(InstrumentError::InstrumentError {
//                error: "unable to determine instrument type".to_string(),
//            })
//        }
//    }
//}

const TEKTRONIX_VID: u16 = 0x0699u16;
const KEITHLEY_VID: u16 = 0x05E6u16;

#[repr(u16)]
#[derive(Clone, Debug, PartialEq, Hash, Eq, Default, serde::Serialize)]
pub enum Vendor {
    #[serde(rename = "TEKTRONIX")]
    Tektronix = TEKTRONIX_VID,

    #[default]
    #[serde(rename = "Keithley Instruments")]
    Keithley = KEITHLEY_VID,
}

impl TryFrom<u16> for Vendor {
    type Error = InstrumentError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            TEKTRONIX_VID => Ok(Self::Tektronix),
            KEITHLEY_VID => Ok(Self::Keithley),
            _ => Err(Self::Error::UnknownVendor(format!(
                "the vendor ID '{value}' is not recognized"
            ))),
        }
    }
}

impl Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Tektronix => "TEKTRONIX",
                Self::Keithley => "Keithley Instruments",
            }
        )
    }
}

impl FromStr for Vendor {
    type Err = InstrumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.to_uppercase().contains("TEKTRONIX") {
            Ok(Self::Tektronix)
        } else if s.to_uppercase().contains("KEITHLEY") {
            Ok(Self::Keithley)
        } else {
            Err(InstrumentError::UnknownVendor(format!(
                "did not recognize vendor '{s}'"
            )))
        }
    }
}

/// This macro is intended only to define instrument models. It should not be used for
/// any other purpose and must not be made public in any way.
///
/// This macro generates an enum with a list of variants that represent all the models
/// this library supports (as well as an `Other` variant which allows the use of unsupported
/// instruments on a best-guess basis). It also implements [`std::str::FromStr`],
/// [`std::fmt::Display`], and [`std::convert::From<u16>`] (for USB product IDs) for
/// each model provided.
macro_rules! define_models {
    ($e_name: ident, $error: ident, $(($name:ident, $string_rep:literal$( | $string_alt:literal)*, $pid:literal$( | $pid_alt:literal)*),)+) => {
        #[derive(Debug, Hash, PartialEq, Eq, Clone, serde::Serialize)]
        pub enum $e_name {
            $($name),+,
            Other(String),
        }

        impl Default for $e_name {
            fn default() -> Self {Self::Other(String::default())}
        }

        impl std::str::FromStr for $e_name {
            type Err = $error;
            fn from_str(val: &str) -> Result<Self, Self::Err> {
                tracing::trace!("{val}");
                match val {
                    $(
                        $string_rep$(| $string_alt)* => Ok($e_name::$name)
                    ),+,
                    _ => Ok($e_name::Other(val.to_string()))
                }
            }
        }

        impl std::fmt::Display for $e_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", match self {
                    $(
                        $e_name::$name => $string_rep
                    ),+,
                    $e_name::Other(s) => s,
                })
            }
        }

        impl std::convert::From<u16> for $e_name {
            fn from(value: u16) -> Self {
                match value {
                    $(
                        $pid$(| $pid_alt)* => $e_name::$name
                    ),+,
                    _ => $e_name::Other(format!("{value}")),
                }
            }
        }
    }
}

define_models! {
    Model,
    InstrumentError,
    //2600
    //(_2601A,),
    //(_2602A,),
    //(_2611A,),
    //(_2612A,),
    //(_2635A,),
    //(_2636A,),
    //(_2651A,),
    //(_2657A,),
    (_2601B, "2601B", 0x2601),
    //(_2601B_PULSE,),
    (_2602B, "2602B", 0x2602),
    (_2606B, "2606B", 0x2606),
    (_2611B, "2611B", 0x2611),
    (_2612B, "2612B", 0x2612),
    (_2635B, "2635B", 0x2635),
    (_2636B, "2636B", 0x2636),
    (_2604B, "2604B", 0x2604),
    (_2614B, "2614B", 0x2614),
    (_2634B, "2634B", 0x2634),
    //(_2601B_L,),
    //(_2602B_L,),
    //(_2611B_L,),
    //(_2612B_L,),
    //(_2635B_L,),
    //(_2636B_L,),
    //(_2604B_L,),
    //(_2614B_L,),
    //(_2634B_L,),

    // 3706 or 70xB
    (_3706A, "3706A", 0x3706),
    //(_3706_SNFP,),
    //(_3706_S,),
    //(_3706_NFP,),
    //(_3706A,),
    //(_3706A_SNFP,),
    //(_3706A_S,),
    //(_3706A_NFP,),
    //(_707B,),
    //(_708B,),
    //(_5880_SRU,),
    //(_5881_SRU,),

    // TTI
    (_2450, "2450", 0x2450),
    (_2470, "2470", 0x2470),
    (DMM7510, "DMM7510", 0x7510),
    (_2460, "2460", 0x2460),
    (_2461, "2461", 0x2461),
    //(_2461_SYS,),
    (DMM7512, "DMM7512", 0x7512),
    (DMM6500, "DMM6500", 0x6500),
    (DAQ6510, "DAQ6510", 0x6510),

    // Modular Platform
    (MP5103, "MP5103" | "TSPop" | "TSP", 0x5103),

}

impl Model {
    #[must_use]
    pub const fn is_tti(&self) -> bool {
        matches!(
            self,
            Self::_2450
                | Self::_2470
                | Self::DMM7510
                | Self::_2460
                | Self::_2461
                //| Self::_2461_SYS
                | Self::DMM7512
                | Self::DMM6500
                | Self::DAQ6510
        )
    }

    #[must_use]
    pub const fn is_mp(&self) -> bool {
        matches!(self, Self::MP5103)
    }

    #[must_use]
    pub const fn is_3700_70x(&self) -> bool {
        matches!(
            self,
            Self::_3706A //| Self::_3706_SNFP
                         //| Self::_3706_S
                         //| Self::_3706_NFP
                         //| Self::_3706A_SNFP
                         //| Self::_3706A_S
                         //| Self::_3706A_NFP
                         //| Self::_707B
                         //| Self::_708B
                         //| Self::_5880_SRU
                         //| Self::_5881_SRU
        )
    }

    #[must_use]
    pub const fn is_2600(&self) -> bool {
        matches!(
            self,
            //| Self::_2602
            //| Self::_2611
            //| Self::_2612
            //| Self::_2635
            //| Self::_2636
            //| Self::_2601A
            //| Self::_2602A
            //| Self::_2611A
            //| Self::_2612A
            //| Self::_2635A
            //| Self::_2636A
            //| Self::_2651A
            //| Self::_2657A
            Self::_2601B
                //| Self::_2601B_PULSE
                | Self::_2602B
                | Self::_2606B
                | Self::_2611B
                | Self::_2612B
                | Self::_2635B
                | Self::_2636B
                | Self::_2604B
                | Self::_2614B
                | Self::_2634B //| Self::_2601B_L
                               //| Self::_2602B_L
                               //| Self::_2611B_L
                               //| Self::_2612B_L
                               //| Self::_2635B_L
                               //| Self::_2636B_L
                               //| Self::_2604B_L
                               //| Self::_2614B_L
                               //| Self::_2634B_L
        )
    }

    #[must_use]
    pub const fn is_other(&self) -> bool {
        matches!(self, Self::Other(_))
    }
}
