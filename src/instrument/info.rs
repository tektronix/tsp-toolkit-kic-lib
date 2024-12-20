//! Define the trait and datatypes necessary to describe an instrument.
use minidom::Element;
use tracing::{debug, instrument, trace};

use crate::{error::Result, InstrumentError};
use std::{
    fmt::Display,
    io::{Read, Write},
    time::Duration,
};

use crate::interface::connection_addr::ConnectionAddr;

/// The information about an instrument.
#[allow(clippy::module_name_repetitions)]
#[derive(serde::Serialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct InstrumentInfo {
    /// The human-readable name of the vendor that makes the instrument
    pub vendor: Option<String>,
    /// The model of the instrument
    pub model: Option<String>,
    /// The serial number of the instrument
    pub serial_number: Option<String>,
    /// The firmware revision of the instrument.
    pub firmware_rev: Option<String>,
    /// The [`ConnectionAddr`] of the instrument.
    #[serde(skip)]
    pub address: Option<ConnectionAddr>,
}

/// Get the [`InstrumentInfo`] from the given object that implements [`Read`] and
/// [`Write`].
///
/// # Errors
/// The following error classes can occur:
/// - Any [`std::io::Error`] that can occur with a [`Read`] or [`Write`] call
/// - Any error in converting the retrieved IDN string into [`InstrumentInfo`]
#[allow(clippy::module_name_repetitions)]
#[instrument(skip(rw))]
pub fn get_info<T: Read + Write + ?Sized>(rw: &mut T) -> Result<InstrumentInfo> {
    debug!("Sending abort");
    rw.write_all(b"abort\n")?;
    std::thread::sleep(Duration::from_millis(100));
    debug!("Sending *CLS");
    rw.write_all(b"*CLS\n")?;
    std::thread::sleep(Duration::from_millis(100));
    debug!("Sending *IDN?");
    rw.write_all(b"*IDN?\n")?;
    let mut info: Option<InstrumentInfo> = None;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(100));

        let mut buf = vec![0u8; 100];
        let _ = rw.read(&mut buf)?;
        let first_null = buf.iter().position(|&x| x == b'\0').unwrap_or(buf.len());
        let buf = &buf[..first_null];
        trace!("Buffer after *IDN?: {}", String::from_utf8_lossy(buf));
        if let Ok(i) = buf.try_into() {
            info = Some(i);
            break;
        }
    }
    info.ok_or_else(|| InstrumentError::InformationRetrievalError {
        details: "unable to read instrument info".to_string(),
    })
}

/// A trait to get the information from an instrument.
pub trait Info: Read + Write {
    /// Get the information for the instrument.
    ///
    /// # Errors
    /// [`TeaspoonInstrumentError::InformationRetrievalError`] if an instrument did not
    /// return the requested information.
    fn info(&mut self) -> Result<InstrumentInfo> {
        get_info(self)
    }
}

impl TryFrom<&[u8]> for InstrumentInfo {
    type Error = InstrumentError;

    fn try_from(idn: &[u8]) -> std::result::Result<Self, Self::Error> {
        let idn = idn
            .iter()
            .position(|&e| e == b'\0')
            .map_or(idn, |first_null| &idn[..first_null]);

        for line in idn.split(|c| *c == b'\n') {
            let parts: Vec<&[u8]> = line.trim_ascii().split(|c| *c == b',').collect();

            if parts.len() != 4 {
                continue;
            }

            match &parts[..] {
                &[v, m, s, f, ..] => {
                    let fw_rev = String::from_utf8_lossy(f)
                        .to_string()
                        .trim_end_matches(|c| c == char::from(0))
                        .trim()
                        .to_string();
                    let (vendor, model, serial_number, firmware_rev) = (
                        Some(String::from_utf8_lossy(v).to_string()),
                        String::from_utf8_lossy(m)
                            .split("MODEL ")
                            .last()
                            .map(std::string::ToString::to_string),
                        Some(String::from_utf8_lossy(s).to_string()),
                        Some(fw_rev),
                    );
                    if model.is_none() {
                        return Err(InstrumentError::InformationRetrievalError {
                            details: "unable to parse model".to_string(),
                        });
                    }

                    return Ok(Self {
                        vendor,
                        model,
                        serial_number,
                        firmware_rev,
                        address: None,
                    });
                }
                _ => {
                    return Err(InstrumentError::InformationRetrievalError {
                        details: "unable to parse instrument information".to_string(),
                    });
                }
            };
        }
        Err(InstrumentError::InformationRetrievalError {
            details: "unable to get instrument information".to_string(),
        })
    }
}

impl TryFrom<&String> for InstrumentInfo {
    type Error = InstrumentError;

    fn try_from(xml_data: &String) -> std::result::Result<Self, Self::Error> {
        const DEVICE_NS: &str = "http://www.lxistandard.org/InstrumentIdentification/1.0";
        if let Ok(root) = xml_data.parse::<Element>() {
            if root.is("LXIDevice", DEVICE_NS) {
                let mut manufacturer = None;
                let mut model_num = None;
                let mut serial_num = None;
                let mut firmware_revision = None;

                let manufacturer_op = root.get_child("Manufacturer", DEVICE_NS);
                let model_op = root.get_child("Model", DEVICE_NS);
                let serial_number_op = root.get_child("SerialNumber", DEVICE_NS);
                let firmware_revision_op = root.get_child("FirmwareRevision", DEVICE_NS);

                if model_op.is_none() {
                    return Err(InstrumentError::InformationRetrievalError {
                        details: "unable to read model".to_string(),
                    });
                }

                if let Some(inst_manufact) = manufacturer_op {
                    manufacturer = Some(inst_manufact.text());
                }

                if let Some(inst_model) = model_op {
                    model_num = Some(inst_model.text());
                }

                if let Some(inst_serial_number) = serial_number_op {
                    serial_num = Some(inst_serial_number.text());
                }

                if let Some(inst_firmware_rev) = firmware_revision_op {
                    firmware_revision = Some(inst_firmware_rev.text());
                }

                return Ok(Self {
                    vendor: manufacturer,
                    model: model_num,
                    serial_number: serial_num,
                    firmware_rev: firmware_revision,
                    address: None,
                });
            }
        }
        Err(InstrumentError::InformationRetrievalError {
            details: "unable to read model".to_string(),
        })
    }
}

// impl tryFrom for string lxi

impl Display for InstrumentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vendor = self.vendor.as_ref().map_or_else(
            || String::from("<UNKNOWN VENDOR>"),
            std::clone::Clone::clone,
        );

        let model: String = self
            .model
            .as_ref()
            .map_or_else(|| String::from("<UNKNOWN MODEL>"), std::clone::Clone::clone);

        let sn: String = self.serial_number.as_ref().map_or_else(
            || String::from("<UNKNOWN SERIAL NUMBER>"),
            std::clone::Clone::clone,
        );

        let fw_rev = self.firmware_rev.as_ref().map_or_else(
            || String::from("<UNKNOWN FIRMWARE REVISION>"),
            std::clone::Clone::clone,
        );

        let addr = self
            .address
            .as_ref()
            .map_or_else(|| ConnectionAddr::Unknown, std::clone::Clone::clone);

        if addr == ConnectionAddr::Unknown {
            write!(f, "{vendor},MODEL {model},{sn},{fw_rev}")
        } else {
            write!(f, "{vendor},MODEL {model},{sn},{fw_rev},{addr}")
        }
    }
}

#[cfg(test)]
mod unit {
    use super::InstrumentInfo;

    #[test]
    fn idn_to_instrument_info_prompts() {
        let input = br"TSP>
KEITHLEY INSTRUMENTS,MODEL 2461,04331961,1.7.12b
TSP>";
        let expected = InstrumentInfo {
            vendor: Some("KEITHLEY INSTRUMENTS".to_string()),
            model: Some("2461".to_string()),
            serial_number: Some("04331961".to_string()),
            firmware_rev: Some("1.7.12b".to_string()),
            address: None,
        };

        let actual = InstrumentInfo::try_from(&input[..]);
        assert!(actual.is_ok(), "Unable to parse InstrumentInfo from &[u8]");

        let actual = actual.unwrap();

        assert_eq!(actual, expected);
    }
}
