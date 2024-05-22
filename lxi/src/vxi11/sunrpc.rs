use super::{Decode, Encode, EnumIdEncode};
use byteorder::{BigEndian, ReadBytesExt};

pub struct Encoder<Data: Encode + Decode + EnumIdEncode> {
    xid: u32,
    data_type: std::marker::PhantomData<Data>,
    program: u32,
    version: u32,
}

pub struct PendingReply {
    pub xid: u32,
    pub program: u32,
    pub version: u32,
    pub procedure: u32,
}

impl<Data: Encode + Decode + EnumIdEncode> Encoder<Data> {
    pub const fn new<const PROGRAM: u32, const VERSION: u32, D: Encode + Decode + EnumIdEncode>(
    ) -> Encoder<D> {
        Encoder::<D> {
            xid: 0,
            data_type: std::marker::PhantomData::<D>,
            program: PROGRAM,
            version: VERSION,
        }
    }

    pub fn call<W: std::io::Write>(&mut self, writer: &mut W, data: Data) -> std::io::Result<PendingReply> {
        self.xid = self.xid.wrapping_add(1);
        let msg = Header::call(
            self.program,
            self.version,
            self.xid,
            data,
            OpaqueAuth {
                flavor: AuthFlavor::None,
            },
            OpaqueAuth {
                flavor: AuthFlavor::None,
            },
        );
        msg.encode(writer)?;
        Ok(PendingReply { xid: self.xid })
    }

    pub fn reply<W: std::io::Write>(
        &mut self,
        writer: &mut W,
        body: ReplyBody<Data>
    ) -> std::io::Result<()> {
        self.xid = self.xid.wrapping_add(1);
        let msg = Header::reply(self.xid, body);
        msg.encode(writer)?;
        Ok(())
    }
}



struct Header<T>
where
    T: Encode + Decode + EnumIdEncode,
{
    xid: u32,
    body: Body<T>,
}

impl<T: Encode + Decode + EnumIdEncode> Encode for Header<T> {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.xid.to_be_bytes());
        message.extend_from_slice(&self.body.variant_id().to_be_bytes());
        message.extend_from_slice(&self.body.to_encoded_bytes());

        message
    }
}

impl<T: Encode + Decode + EnumIdEncode> Header<T> {
    #[must_use]
    const fn call(
        program: u32,
        version: u32,
        xid: u32,
        data: T,
        credentials: OpaqueAuth,
        verifier: OpaqueAuth,
    ) -> Self {
        Self {
            xid,
            body: Body::Call(CallBody {
                rpc_version: 2,
                program,
                version,
                credentials,
                verifier,
                data,
            }),
        }
    }

    fn reply(xid: u32, body: ReplyBody<T>) -> Self {
        Self {
            xid,
            body: Body::Reply(body),
        }
    }
}

#[repr(u32)]
enum Body<T>
where
    T: Encode + Decode + EnumIdEncode,
{
    Call(CallBody<T>) = 0u32,
    Reply(ReplyBody<T>) = 1u32,
}

impl<T: Encode + Decode + EnumIdEncode> EnumIdEncode for Body<T> {}

impl<T: Encode + Decode + EnumIdEncode> Encode for Body<T> {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        match self {
            Self::Call(c) => c.to_encoded_bytes(),
            Self::Reply(r) => r.to_encoded_bytes(),
        }
    }
}

struct CallBody<T>
where
    T: Encode + Decode + EnumIdEncode,
{
    rpc_version: u32,
    program: u32,
    version: u32,
    credentials: OpaqueAuth,
    verifier: OpaqueAuth,
    data: T,
}

impl<T: Encode + Decode + EnumIdEncode> Encode for CallBody<T> {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.rpc_version.to_be_bytes());
        message.extend_from_slice(&self.program.to_be_bytes());
        message.extend_from_slice(&self.version.to_be_bytes());
        message.extend_from_slice(&self.data.variant_id().to_be_bytes());
        message.extend_from_slice(&self.credentials.to_encoded_bytes());
        message.extend_from_slice(&self.verifier.to_encoded_bytes());
        message.extend_from_slice(&self.data.to_encoded_bytes());

        message
    }
}

impl<T: Encode + Decode + EnumIdEncode> Decode for CallBody<T> {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let rpc_version: u32 = reader.read_u32::<BigEndian>()?;
        let program: u32 = reader.read_u32::<BigEndian>()?;
        let version: u32 = reader.read_u32::<BigEndian>()?;
        let credentials: OpaqueAuth = OpaqueAuth::decode(reader)?;
        let verifier: OpaqueAuth = OpaqueAuth::decode(reader)?;
        let data = T::decode(reader)?;

        Ok(Self {
            rpc_version,
            program,
            version,
            credentials,
            verifier,
            data,
        })
    }
}

#[repr(u32)]
pub enum ReplyBody<T: Encode + Decode + EnumIdEncode> {
    Accepted(AcceptedReplyBody<T>) = 0,
    Rejected(RejectedReplyBody) = 1,
}

impl<T: Encode + Decode + EnumIdEncode> EnumIdEncode for ReplyBody<T> {}

impl<T: Encode + Decode + EnumIdEncode> Encode for ReplyBody<T> {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        match self {
            Self::Accepted(a) => {
                message.extend_from_slice(&self.variant_id().to_be_bytes());
                message.extend_from_slice(&a.to_encoded_bytes());
            }
            Self::Rejected(r) => {
                message.extend_from_slice(&self.variant_id().to_be_bytes());
                message.extend_from_slice(&r.to_encoded_bytes());
            }
        }

        message
    }
}

impl<T: Encode + Decode + EnumIdEncode> Decode for ReplyBody<T> {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let branch = reader.read_u32::<BigEndian>()?;
        Ok(match branch {
            0 => Self::Accepted(AcceptedReplyBody::decode(reader)?),
            1 => Self::Rejected(RejectedReplyBody::decode(reader)?),
            _ => {
                return Err(super::Error::DecodeError(format!(
                    "unknown accept/reject state {branch}"
                )))
            }
        })
    }
}

pub struct AcceptedReplyBody<T: Encode + Decode + EnumIdEncode> {
    verifier: OpaqueAuth,
    data: AcceptedReplyData<T>,
}

impl<T: Encode + Decode + EnumIdEncode> Encode for AcceptedReplyBody<T> {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.verifier.to_encoded_bytes());
        message.extend_from_slice(&self.data.variant_id().to_be_bytes());
        message.extend_from_slice(&self.data.to_encoded_bytes());

        message
    }
}

impl<T: Encode + Decode + EnumIdEncode> Decode for AcceptedReplyBody<T> {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let verifier: OpaqueAuth = OpaqueAuth::decode(reader)?;
        let data: AcceptedReplyData<T> = AcceptedReplyData::decode(reader)?;

        Ok(Self { verifier, data })
    }
}

#[repr(u32)]
pub enum AcceptedReplyData<T: Encode + Decode + EnumIdEncode> {
    Success(T) = 0,
    ProgramUnavailable = 1,
    ProgramMismatch { low: u32, high: u32 } = 2,
    ProcedureUnavailable = 3,
    GarbageArgs = 4,
}

impl<T: Encode + Decode + EnumIdEncode> EnumIdEncode for AcceptedReplyData<T> {}

impl<T: Encode + Decode + EnumIdEncode> Encode for AcceptedReplyData<T> {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        match self {
            Self::ProgramMismatch { low, high } => {
                let mut message = Vec::new();
                message.extend_from_slice(&low.to_be_bytes());
                message.extend_from_slice(&high.to_be_bytes());

                message
            }
            Self::Success(d) => d.to_encoded_bytes(),
            Self::ProgramUnavailable
            | Self::ProcedureUnavailable
            | Self::GarbageArgs => Vec::new(),
        }
    }
}

impl<T: Encode + Decode + EnumIdEncode> Decode for AcceptedReplyData<T> {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let variant = reader.read_u32::<BigEndian>()?;
        Ok(match variant {
            0 => {
                let d: T = T::decode(reader)?;
                Self::Success(d)
            }
            1 => Self::ProgramUnavailable,
            2 => {
                let low = reader.read_u32::<BigEndian>()?;
                let high = reader.read_u32::<BigEndian>()?;
                Self::ProgramMismatch { low, high }
            }
            3 => Self::ProcedureUnavailable,
            4 => Self::GarbageArgs,
            _ => {
                return Err(super::Error::DecodeError(format!(
                    "unknown accept state {variant}"
                )))
            }
        })
    }
}

#[repr(u32)]
pub enum RejectedReplyBody {
    RpcMismatch { low: u32, high: u32 } = 0,
    AuthenticationError { state: AuthenticationState } = 1,
}

impl Encode for RejectedReplyBody {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        match self {
            Self::RpcMismatch { low, high } => {
                message.extend_from_slice(&low.to_be_bytes());
                message.extend_from_slice(&high.to_be_bytes());
            }
            Self::AuthenticationError { state } => {
                message.extend_from_slice(&state.variant_id().to_be_bytes());
            }
        }

        message
    }
}

impl Decode for RejectedReplyBody {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let branch = reader.read_u32::<BigEndian>()?;
        Ok(match branch {
            0 => {
                let low = reader.read_u32::<BigEndian>()?;
                let high = reader.read_u32::<BigEndian>()?;
                Self::RpcMismatch { low, high }
            }
            1 => {
                let auth_error_state = reader.read_u32::<BigEndian>()?;
                let auth_error_state = match auth_error_state {
                    1 => AuthenticationState::BadCredentials,
                    2 => AuthenticationState::RejectedCredentials,
                    3 => AuthenticationState::BadVerifier,
                    4 => AuthenticationState::RejectedVerifier,
                    5 => AuthenticationState::TooWeak,
                    _ => {
                        return Err(super::Error::DecodeError(format!(
                            "unknown authentication state {auth_error_state}"
                        )))
                    }
                };
                Self::AuthenticationError {
                    state: auth_error_state,
                }
            }
            _ => {
                return Err(super::Error::DecodeError(format!(
                    "unknown reject state {branch}"
                )))
            }
        })
    }
}

impl EnumIdEncode for RejectedReplyBody {}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum AuthenticationState {
    BadCredentials = 1,
    RejectedCredentials = 2,
    BadVerifier = 3,
    RejectedVerifier = 4,
    TooWeak = 5,
}

impl EnumIdEncode for AuthenticationState {}

#[derive(Default)]
struct OpaqueAuth {
    flavor: AuthFlavor,
}

impl Encode for OpaqueAuth {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        let flavor = self.flavor.variant_id();
        let length: u32 = 0;

        message.extend_from_slice(&flavor.to_be_bytes());
        message.extend_from_slice(&length.to_be_bytes());

        message
    }
}

impl Decode for OpaqueAuth {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let _flavor = reader.read_u32::<BigEndian>()?;
        let _length = reader.read_u32::<BigEndian>()?;
        Ok(Self {
            flavor: AuthFlavor::None,
        })
    }
}

/// The type of authentication used by the device with which we are
/// communicating. Keithley doesn't support any of these, as far as I can tell
/// so we will just support [`AuthFlavor::None`] for now.
#[repr(u32)]
#[derive(Default, Debug, Clone, Copy)]
pub enum AuthFlavor {
    /// No authentication used/required.
    #[default]
    None = 0,
    //System  = 1,
    //Short = 2,
    //Des = 3,
    //Kerberos = 4,
}

impl EnumIdEncode for AuthFlavor {}

impl Encode for AuthFlavor {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        match self {
            Self::None => Vec::new(),
        }
    }
}

#[cfg(test)]
mod unit {
    use crate::vxi11::{
        sunrpc::{Header, OpaqueAuth},
        Decode, Encode, EnumIdEncode,
    };
    use byteorder::{BigEndian, ReadBytesExt};

    const PROGRAM: u32 = 395_183; //VXI-11
    const PROGRAM_VERSION: u32 = 1;

    #[repr(u32)]
    enum Test {
        A = 10,
        B(u8) = 11,
        C { d: String } = 15,
    }
    impl EnumIdEncode for Test {}

    impl Encode for Test {
        fn to_encoded_bytes(&self) -> Vec<u8> {
            let mut bytes = Vec::new();
            match self {
                Self::A => {}
                Self::B(x) => {
                    bytes.extend_from_slice(&[0u8, 0, 0, *x]);
                }
                Self::C { d } => {
                    bytes.extend_from_slice(&[0u8, 0, 0]);
                    bytes.extend_from_slice(d.as_bytes());
                }
            }

            bytes
        }
    }

    impl Decode for Test {
        fn decode<R: std::io::Read>(reader: &mut R) -> crate::vxi11::Result<Self> {
            let branch = reader.read_u32::<BigEndian>()?;
            Ok(match branch {
                10 => Self::A,
                11 => {
                    let _len = reader.read_u32::<BigEndian>()?;
                    let b = reader.read_u8()?;
                    let _ = reader.read_u8()?;
                    let _ = reader.read_u8()?;
                    let _ = reader.read_u8()?;
                    Self::B(b)
                }
                15 => {
                    let len = reader.read_u32::<BigEndian>()?;
                    let mut buf: Vec<u8> = vec![0; len.try_into().unwrap()];
                    reader.read_exact(&mut buf)?;
                    let d = String::from_utf8_lossy(&buf).into_owned();
                    Self::C { d }
                }
                _ => {
                    return Err(crate::vxi11::Error::DecodeError(format!(
                        "unknown procedure number {branch}"
                    )))
                }
            })
        }
    }

    #[test]
    fn call_to_bytes_no_proc() {
        let msg = Header::<Test>::call(
            PROGRAM,
            PROGRAM_VERSION,
            1,
            Test::A,
            OpaqueAuth::default(),
            OpaqueAuth::default(),
        );

        let actual = msg.to_encoded_bytes();

        let expected = [
            //Header
            0x00, 0x00, 0x00, 0x01, // xid: 1
            0x00, 0x00, 0x00, 0x00, // MessageType: Call
            0x00, 0x00, 0x00, 0x02, // rpc version: 2
            0x00, 0x06, 0x07, 0xAF, // program: 395183 == 0x000607af
            0x00, 0x00, 0x00, 0x01, // program version: 1
            0x00, 0x00, 0x00, 0x00, // procedure: None == 0
            0x00, 0x00, 0x00, 0x00, // Cred Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
            0x00, 0x00, 0x00, 0x00, // Verifier Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn call_to_bytes_with_type_only_proc() {
        let msg = Header::<Test>::call(
            PROGRAM,
            PROGRAM_VERSION,
            1,
            Test::A,
            OpaqueAuth::default(),
            OpaqueAuth::default(),
        );

        let actual = msg.to_encoded_bytes();

        let expected = [
            //Header
            0x00, 0x00, 0x00, 0x01, // xid: 1
            0x00, 0x00, 0x00, 0x00, // MessageType: Call
            0x00, 0x00, 0x00, 0x02, // rpc version: 2
            0x00, 0x06, 0x07, 0xAF, // program: 395183 == 0x000607af
            0x00, 0x00, 0x00, 0x01, // program version: 1
            0x00, 0x00, 0x00, 0x0A, // procedure: Test:A == 10
            0x00, 0x00, 0x00, 0x00, // Cred Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
            0x00, 0x00, 0x00, 0x00, // Verifier Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn call_to_bytes_with_tuple_proc() {
        let msg = Header::<Test>::call(
            PROGRAM,
            PROGRAM_VERSION,
            1,
            Test::B(0xFE),
            OpaqueAuth::default(),
            OpaqueAuth::default(),
        );

        let actual = msg.to_encoded_bytes();

        let expected = [
            //Header
            0x00, 0x00, 0x00, 0x01, // xid: 1
            0x00, 0x00, 0x00, 0x00, // MessageType: Call
            0x00, 0x00, 0x00, 0x02, // rpc version: 2
            0x00, 0x06, 0x07, 0xAF, // program: 395183 == 0x000607af
            0x00, 0x00, 0x00, 0x01, // program version: 1
            0x00, 0x00, 0x00, 0x0B, // procedure: Test::B == 11
            0x00, 0x00, 0x00, 0x00, // Cred Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
            0x00, 0x00, 0x00, 0x00, // Verifier Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
            0x00, 0x00, 0x00, 0xFE, // Test:B(0xFE)
        ];

        assert_eq!(actual, expected);
    }

    #[test]
    fn call_to_bytes_with_struct_proc() {
        let msg = Header::<Test>::call(
            PROGRAM,
            PROGRAM_VERSION,
            1,
            Test::C {
                d: "Hello".to_string(),
            },
            OpaqueAuth::default(),
            OpaqueAuth::default(),
        );

        let actual = msg.to_encoded_bytes();

        let expected = [
            //Header
            0x00, 0x00, 0x00, 0x01, // xid: 1
            0x00, 0x00, 0x00, 0x00, // MessageType: Call
            0x00, 0x00, 0x00, 0x02, // rpc version: 2
            0x00, 0x06, 0x07, 0xAF, // program: 395183 == 0x000607af
            0x00, 0x00, 0x00, 0x01, // program version: 1
            0x00, 0x00, 0x00, 0x0F, // procedure: Test::C == 15
            0x00, 0x00, 0x00, 0x00, // Cred Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
            0x00, 0x00, 0x00, 0x00, // Verifier Auth Flavor: NULL
            0x00, 0x00, 0x00, 0x00, // Length: 0
            0x00, 0x00, 0x00, 0x48, // Test:C { d: "Hello"}
            0x65, 0x6C, 0x6C, 0x6F,
        ];

        assert_eq!(actual, expected);
    }
}
