//! All the errors that this crate can emit are defined in the
//! [`error::InstrumentError`] enum.

use std::{num::ParseIntError, string::FromUtf8Error};

use thiserror::Error;

use crate::interface::connection_addr::ConnectionAddr;

/// Define errors that originate from this crate
#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum InstrumentError {
    /// The `unparsable_string` was passed where an address was expected, but
    /// it couldn't be parsed to a valid address.
    #[error("unable to parse `{unparsable_string}`, expected an address")]
    AddressParsingError {
        ///The string that couldn't be parsed
        unparsable_string: String,
    },

    /// The [`ConnectionAddr`] was not able to be converted to the desired device
    /// interface type
    #[error("unable to convert from `{from:?}` to {to}")]
    ConnectionAddressConversionError {
        /// The address information trying to be converted from
        from: ConnectionAddr,
        /// A string of name of the type trying to be converted to
        to: String,
    },

    /// There was an error while trying to connect to the interface or instrument.
    #[error("connection error occurred: {details}")]
    ConnectionError {
        // The details of the [`ConnectionError`]
        details: String,
    },

    /// There was an issue while disconnecting from an instrument.
    #[error("unable to gracefully disconnect from instrument: {details}")]
    DisconnectError {
        /// More information about the disconnection error.
        details: String,
    },

    /// A resource file was unable to be decrypted.
    #[error("unable to decrypt resource: {source}")]
    ResourceDecryptError {
        ///**TODO:** Change this to the error that is produced when decryption fails
        #[from]
        source: FromUtf8Error,
    },

    /// An error that occurs while trying to retrieve information about an instrument
    /// such as the serial number, model, manufacturer, etc.
    #[error("instrument information retrieval error: {details}")]
    InformationRetrievalError {
        /// Any extra information about why the instrument information could not be
        /// retrieved.
        details: String,
    },

    /// An Error that originates from an instrument. This is generic for all instruments
    /// and is therefore just a [`String`].
    #[error("{error}")]
    InstrumentError {
        /// The error string provided by the instrument.
        error: String,
    },

    /// Converts a [`std::io::Error`] to a [`TeaspoonInterfaceError`]
    #[error("IO error: {source}")]
    IoError {
        /// The [`std::io::Error`] from which this [`TeaspoonInterfaceError::IoError`]
        /// was derived.
        #[from]
        source: std::io::Error,
    },

    /// The provided login details were either incorrect or the instrument is already
    /// claimed and cannot be claimed again.
    #[error("provided login details rejected or instrument already claimed.")]
    LoginRejected,

    /// The instrument is already claimed by another interface.
    #[error("Another interface has control, logout on that interface.")]
    InterfaceLoginErr,

    #[error("{source}")]
    ParseIntError {
        #[from]
        source: ParseIntError,
    },

    /// The TSP error that was received from the instrument was malformed.
    #[error("unable to parse TSP error from instrument {error}")]
    TspErrorParseError {
        /// The text of the malformed error that was provided by the instrument
        error: String,
    },

    /// The queried instrument returned an unknown model number
    #[error("\"{model}\" is not a recognized model number")]
    UnknownInstrumentModel {
        /// The unknown model number
        model: String,
    },

    /// The queried instrument returned an unknown language type
    #[error("\"{lang} is not a recognized embedded instrument language\"")]
    UnknownLanguage {
        /// The unknown language type
        lang: String,
    },

    #[cfg(feature = "visa")]
    /// An error from the visa driver
    #[error("visa error: {source}")]
    VisaError {
        #[from]
        source: visa_rs::Error,
    },

    /// An uncategorized error.
    #[error("{0}")]
    Other(String),
}

pub(crate) type Result<T> = std::result::Result<T, InstrumentError>;
