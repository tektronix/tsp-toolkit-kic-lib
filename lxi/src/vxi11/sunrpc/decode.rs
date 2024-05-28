use std::io::{BufReader, Cursor};

use byteorder::{BigEndian, ReadBytesExt};

use crate::vxi11::{Encode, EnumIdEncode, ProcDecode};

use super::{error::Error, Header};

pub struct Decoder<Data: Encode + ProcDecode + EnumIdEncode> {
    pub data_type: std::marker::PhantomData<Data>,
    pub program: u32,
    pub version: u32,
}

impl<Data: Encode + ProcDecode + EnumIdEncode + Clone + PartialEq + Eq> Decoder<Data> {
    pub const fn new<
        const PROGRAM: u32,
        const VERSION: u32,
        D: Encode + ProcDecode + EnumIdEncode,
    >() -> Decoder<D> {
        Decoder::<D> {
            data_type: std::marker::PhantomData::<D>,
            program: PROGRAM,
            version: VERSION,
        }
    }

    pub fn peek_xid<R>(reader: &mut R) -> super::error::Result<u32>
    where
        R: std::io::Read,
    {
        let mut cursor = Cursor::new(reader);

        let xid = cursor.get_mut().read_u32::<BigEndian>()?;

        cursor.set_position(0);

        Ok(xid)
    }

    /// Sun/XDR RPC requires that you keep track of the procedure ID for the given
    /// reply message yourself (it isn't given with the reply).
    pub fn proc_decode<R: std::io::Read>(
        reader: &mut R,
        procedure_id: u32,
    ) -> super::error::Result<Data> {
        match Header::<Data>::proc_decode(reader, procedure_id) {
            Ok(o) => match o.body {
                super::Body::Call(c) => Ok(c.data),
                super::Body::Reply(reply) => match reply {
                    super::ReplyBody::Accepted(a) => match a.data {
                        super::AcceptedReplyData::Success(s) => Ok(s),
                        super::AcceptedReplyData::ProgramUnavailable => {
                            Err(Error::ProgramUnavailable)
                        }
                        super::AcceptedReplyData::ProgramMismatch { low, high } => {
                            Err(Error::ProgramMismatch { low, high })
                        }
                        super::AcceptedReplyData::ProcedureUnavailable => {
                            Err(Error::ProcedureUnavailable)
                        }
                        super::AcceptedReplyData::GarbageArgs => Err(Error::GarbageArgs),
                    },
                    super::ReplyBody::Rejected(rej) => match rej {
                        super::RejectedReplyBody::RpcMismatch { low, high } => {
                            Err(Error::RpcMismatch { low, high })
                        }
                        super::RejectedReplyBody::AuthenticationError { state } => {
                            Err(match state {
                                super::AuthenticationState::TooWeak => Error::AuthenticationTooWeak,
                                super::AuthenticationState::BadVerifier => Error::BadVerifier,
                                super::AuthenticationState::BadCredentials => Error::BadCredentials,
                                super::AuthenticationState::RejectedVerifier => {
                                    Error::RejectedVerifier
                                }
                                super::AuthenticationState::RejectedCredentials => {
                                    Error::RejectedCredentials
                                }
                            })
                        }
                    },
                },
            },
            Err(e) => Err(Error::Other(e.to_string())),
        }
    }
}
