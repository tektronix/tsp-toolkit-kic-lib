use crate::{
    instrument::{authenticate::Authenticate, Instrument},
    InstrumentError, Interface,
};

pub mod ki2600;
pub mod tti;
pub mod versatest;

impl TryFrom<Box<dyn Interface>> for Box<dyn Instrument> {
    type Error = InstrumentError;

    fn try_from(mut interface: Box<dyn Interface>) -> std::result::Result<Self, Self::Error> {
        let info = interface.as_mut().info()?;
        let auth = Box::new(Authenticate {});
        if tti::Instrument::is(&info) {
            let mut ins = Box::new(tti::Instrument::new(interface, auth));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if ki2600::Instrument::is(&info) {
            let mut ins = Box::new(ki2600::Instrument::new(interface, auth));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else if versatest::Instrument::is(&info) {
            let mut ins = Box::new(versatest::Instrument::new(interface, auth));
            ins.as_mut().add_info(info);
            Ok(ins)
        } else {
            Err(InstrumentError::InstrumentError {
                error: "unable to determine instrument type".to_string(),
            })
        }
    }
}
