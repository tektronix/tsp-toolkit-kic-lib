use std::io::{Read, Write};

use crate::error::Result;
use crate::interface::async_stream::StatusByte;

/// Determine whether the device is still active.
/// For [`crate::instrument::Instrument`]s, this will be a call through to the
/// underlying [`crate::interface::Interface`] whereas for the
/// [`crate::interface::Interface`], the implementation will vary.
pub trait Active: Read + Write {
    /// Determine whether the connection to this device is still active.
    /// `false` is returned in the case of any IO errors, and `true` is only
    /// returned if the device responds in an appropriate fashion to polling.
    ///
    /// # Implementation
    /// Any implementation of this function must use a polling method that does
    /// _not_ interrupt the current operation of the device.
    fn is_active(&mut self) -> bool {
        self.get_status().is_ok()
    }

    /// Get the status byte (STB) of the device and return it as a [`StatusByte`]
    ///
    /// # Errors
    /// This will return an error with any IO errors communicating with the
    /// device.
    fn get_status(&mut self) -> Result<StatusByte>;
}
