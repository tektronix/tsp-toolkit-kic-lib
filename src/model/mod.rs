use crate::{
    instrument::{authenticate::Authenticate, Info, Instrument},
    protocol::{self, Protocol},
    InstrumentError, Interface,
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

impl TryFrom<Protocol> for Box<dyn Instrument> {
    type Error = InstrumentError;

    fn try_from(mut proto: Protocol) -> Result<Self, Self::Error> {
        let info = proto.info()?;
        let auth = Box::new(Authenticate {});
        if tti::Instrument::is(&info) {
            let mut ins = Box::new(tti::Instrument::new(proto, auth));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if ki2600::Instrument::is(&info) {
            let mut ins = Box::new(ki2600::Instrument::new(proto, auth));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if ki3700::Instrument::is(&info) {
            let mut ins = Box::new(ki3700::Instrument::new(proto, auth));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if versatest::Instrument::is(&info) {
            let mut ins = Box::new(versatest::Instrument::new(proto, auth));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else {
            Err(InstrumentError::InstrumentError {
                error: "unable to determine instrument type".to_string(),
            })
        }
    }
}

impl TryFrom<Box<dyn Interface>> for Box<dyn Instrument> {
    type Error = InstrumentError;

    fn try_from(mut interface: Box<dyn Interface>) -> std::result::Result<Self, Self::Error> {
        let info = interface.as_mut().info()?;
        let auth = Box::new(Authenticate {});
        if tti::Instrument::is(&info) {
            let mut ins = Box::new(tti::Instrument::new(
                protocol::Protocol::Raw(interface),
                auth,
            ));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if ki2600::Instrument::is(&info) {
            let mut ins = Box::new(ki2600::Instrument::new(
                protocol::Protocol::Raw(interface),
                auth,
            ));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if ki3700::Instrument::is(&info) {
            let mut ins = Box::new(ki3700::Instrument::new(
                protocol::Protocol::Raw(interface),
                auth,
            ));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if versatest::Instrument::is(&info) {
            let mut ins = Box::new(versatest::Instrument::new(
                protocol::Protocol::Raw(interface),
                auth,
            ));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else {
            Err(InstrumentError::InstrumentError {
                error: "unable to determine instrument type".to_string(),
            })
        }
    }
}
