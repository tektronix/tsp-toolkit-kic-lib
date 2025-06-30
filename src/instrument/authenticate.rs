// Authenticate functionality of the instrument.

use crate::InstrumentError;

/// An enum that provides the expected functionality for authentication into an instrument.
///
/// Some instruments may not have the concept of authentication in, for those instruments,
/// simply pass `Authentiate` struct to the instrument and do not use it in Login trait.
///
/// Note that none of the functions or implementing structures have built-in
/// cryptography and are therefore not inherently secure. Communication with most
/// instruments is also done in plain text and therefore it seems disingenuous to
/// focus on security here.
pub enum Authentication {
    /// Prompt the user on the commandline for their username (visible) and password (hidden).
    Prompt,
    /// Allows a credential to be entered without prompts or keyring lookups.
    Credential { username: String, password: String },
    /// Uses an id to look up the proper credentials in the system keyring.
    Keyring { id: String },
    /// No authentication is required, don't try to use any.
    NoAuth,
}

impl Authentication {
    ///
    /// Retrieves the username
    ///
    /// # Errors
    ///
    /// Returns an error if the retrieval fails.
    ///
    pub fn read_username(&self) -> Result<Option<String>, InstrumentError> {
        match self {
            Self::Prompt => {
                eprintln!("Enter Username:");
                let mut username = String::new();
                std::io::stdin().read_line(&mut username)?;
                let username = username.trim();
                Ok(Some(username.to_string()))
            }
            Self::Credential { username, .. } => Ok(Some(username.clone())),
            Self::Keyring { id: _id } => todo!(),
            Self::NoAuth => Ok(None),
        }
    }

    ///
    /// Retrieves the password
    ///
    /// # Errors
    ///
    /// Returns an [`std::io::Error`] if any errors occurred.
    ///
    pub fn read_password(&self) -> Result<Option<String>, InstrumentError> {
        match self {
            Self::Prompt => {
                eprintln!("Enter Password (characters hidden):");
                Ok(Some(rpassword::read_password()?))
            }
            Self::Credential { password, .. } => Ok(Some(password.clone())),
            Self::Keyring { id: _id } => todo!(),
            Self::NoAuth => Ok(None),
        }
    }
}
