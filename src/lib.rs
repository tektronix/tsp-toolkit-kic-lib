#![feature(lint_reasons, rustdoc_missing_doc_code_examples)]
#![feature(assert_matches)]
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
