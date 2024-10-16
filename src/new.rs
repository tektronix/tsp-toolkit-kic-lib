pub mod instrument;
pub mod protocol;

use std::{error::Error, fmt::Display};

use instrument::model::Model;

use crate::{ConnectionAddr, InstrumentError};

/// The information about an instrument.
#[allow(clippy::module_name_repetitions)]
#[derive(serde::Serialize, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct InstrumentInfo {
    /// The human-readable name of the vendor that makes the instrument
    pub vendor: Option<String>,
    /// The model of the instrument
    pub model: Option<Model>,
    /// The serial number of the instrument
    pub serial_number: Option<String>,
    /// The firmware revision of the instrument.
    pub firmware_rev: Option<String>,
    /// The [`ConnectionAddr`] of the instrument.
    #[serde(skip)]
    pub address: Option<ConnectionAddr>,
}

impl Display for InstrumentInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let vendor = self.vendor.as_ref().map_or_else(
            || String::from("<UNKNOWN VENDOR>"),
            std::clone::Clone::clone,
        );

        let model: String = self
            .model
            .as_ref()
            .map_or_else(|| String::from("<UNKNOWN MODEL>"), Model::to_string);

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

/// The trait an instrument must implement in order to flash the firmware onto an
/// instrument.
pub trait Flash {
    type Error: Display + Error;
    /// The method to flash a firmware image to an instrument.
    ///
    /// # Errors
    /// An error can occur in the write to or reading from the instrument as well as in
    /// reading the firmware image.
    fn flash_firmware(&mut self, image: &[u8]) -> core::result::Result<(), Self::Error>;

    /// The method to flash a firmware image to an instrument's module
    ///
    /// # Errors
    /// An error can occur in the write to or reading from the instrument as well as in
    /// reading the firmware image.
    /// May also emit an error if the connected instrument doesn't have modules.
    fn flash_module(&mut self, module: u16, image: &[u8]) -> core::result::Result<(), Self::Error>;
}

/// The [`Instrument`] can write a script to be executed.
pub trait Script {
    type Error: Display + Error;

    /// Write the given script to the instrument with the given name.
    ///
    /// # Parameters
    /// - `name` - is the environment-compatible name of the script
    /// - `script` - is the contents of the script
    /// - `save_script` - `true` if the script should be saved to non-volatile memory
    /// - `run_script` - `true` if the script should be run after load
    ///
    /// # Notes
    /// - The script name will not be validated to ensure that it is compatible with the
    ///     scripting environment.
    /// - The given script content will only be validated by the instrument, but not
    ///     the [`write_script`] function.
    ///
    /// # Errors
    /// Returns an [`InstrumentError`] if any errors occurred.
    fn write_script(
        &mut self,
        name: &[u8],
        script: &[u8],
        save_script: bool,
        run_script: bool,
    ) -> core::result::Result<(), Self::Error>;
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

impl TryFrom<&[u8]> for InstrumentInfo {
    type Error = InstrumentError;

    fn try_from(idn: &[u8]) -> std::result::Result<Self, Self::Error> {
        let parts: Vec<&[u8]> = idn
            .split(|c| *c == b',' || *c == b'\n' || *c == b'\0')
            .collect();

        let (vendor, model, serial_number, firmware_rev) = match &parts[..] {
            &[v, m, s, f, ..] => {
                let fw_rev = String::from_utf8_lossy(f)
                    .to_string()
                    .trim_end_matches(|c| c == char::from(0))
                    .trim()
                    .to_string();
                (
                    Some(String::from_utf8_lossy(v).to_string()),
                    Some(
                        String::from_utf8_lossy(m)
                            .split("MODEL ")
                            .last()
                            .map_or_else(|| "UNKNOWN".to_string(), std::string::ToString::to_string)
                            .parse::<Model>()?,
                    ),
                    Some(String::from_utf8_lossy(s).to_string()),
                    Some(fw_rev),
                )
            }
            _ => {
                return Err(InstrumentError::InformationRetrievalError {
                    details: "unable to parse instrument information".to_string(),
                });
            }
        };

        if model.is_none() {
            return Err(InstrumentError::InformationRetrievalError {
                details: "unable to parse model".to_string(),
            });
        }

        Ok(Self {
            vendor,
            model,
            serial_number,
            firmware_rev,
            address: None,
        })
    }
}
