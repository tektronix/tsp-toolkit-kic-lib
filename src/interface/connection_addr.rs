use std::fmt::Display;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use visa_rs::DefaultRM;
#[cfg(feature = "visa")]
use visa_rs::{AsResourceManager, VisaString};

use crate::InstrumentError;

/// A generic connection address that covers all the different connection types.
///
/// Each device interface type will also have a [`TryFrom`] impl that converts from
/// this enum to itself. [`From`] is **not** implemented because the conversion could
/// fail.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConnectionAddr {
    /// A LAN connection is created with a [`SocketAddr`], which includes an [`IpAddr`] and
    /// a port for the connection.
    Lan(SocketAddr),

    #[cfg(feature = "visa")]
    /// A VISA resource string
    Visa(VisaString),

    //Add other device interface types here
    Unknown,
}

impl FromStr for ConnectionAddr {
    type Err = InstrumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(i) = s.parse::<SocketAddr>() {
            return Ok(Self::Lan(i));
        }
        if let Ok(i) = s.parse::<IpAddr>() {
            return Ok(Self::Lan(SocketAddr::new(i, 0)));
        }
        #[cfg(feature = "visa")]
        {
            let rm = DefaultRM::new()?;
            let res_id = visa_rs::ResID::from_string(s.to_string());
            if let Some(ri) = res_id {
                if rm.parse_res_ex(&ri).is_ok() {
                    return Ok(Self::Visa(ri));
                }
            }
        }
        Err(InstrumentError::Other(
            "unable to parse {s} to a valid connection string".to_string(),
        ))
    }
}

impl Display for ConnectionAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Lan(lan_info) => lan_info.to_string(),

            #[cfg(feature = "visa")]
            Self::Visa(visa_info) => visa_info.to_string(),

            Self::Unknown => "<UNKNOWN>".to_string(),
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod unit {
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

    use visa_rs::VisaString;

    use super::ConnectionAddr;

    #[test]
    fn ipv4_addr_str_to_conn_addr() -> anyhow::Result<()> {
        let test = "192.168.0.1";

        let actual = test.parse::<ConnectionAddr>()?;

        let expected =
            ConnectionAddr::Lan(SocketAddr::new(Ipv4Addr::new(192, 168, 0, 1).into(), 5025));

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn ipv6_addr_str_to_conn_addr() -> anyhow::Result<()> {
        let test = "2345:0425:2CA1:0000:0000:0567:5673:23b5";

        let actual = test.parse::<ConnectionAddr>()?;

        let expected = ConnectionAddr::Lan(SocketAddr::new(
            Ipv6Addr::new(
                0x2345, 0x0425, 0x2CA1, 0x0000, 0x0000, 0x0567, 0x5673, 0x23b5,
            )
            .into(),
            5025,
        ));

        assert_eq!(actual, expected);

        Ok(())
    }

    #[cfg(feature = "visa")]
    #[test]
    fn visa_tcpip_res_str_to_conn_addr() -> anyhow::Result<()> {
        let test = "TCPIP0::1.2.3.4::inst0::INSTR";

        let actual = test.parse::<ConnectionAddr>()?;

        let expected = ConnectionAddr::Visa(
            VisaString::from_string("TCPIP0::1.2.3.4::inst0::INSTR".to_string())
                .expect("should convert to visa string"),
        );

        assert_eq!(actual, expected);

        Ok(())
    }
}
