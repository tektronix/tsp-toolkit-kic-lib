// Authenticate functionality of the instrument.

/// A trait that provides the expected functionality for authentication into an instrument.
///
/// This trait is required for Authenticate struct to be moved into the instrument.
///
/// Some instruments may not have the concept of authentication in, for those instruments,
/// simply pass `Authentiate` struct to the instrument and do not use it in Login trait.
pub trait Authentication {
    ///
    /// Prompts user for a password.
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if any errors occurred.
    ///
    fn read_password(&self) -> std::io::Result<String>;
}

pub struct Authenticate {}

impl Authentication for Authenticate {
    fn read_password(&self) -> std::io::Result<String> {
        rpassword::read_password()
    }
}
