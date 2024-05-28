use std::{
    collections::HashMap,
    io::{Read, Write},
};

use crate::vxi11::{Encode, EnumIdEncode, ProcDecode, Result};

use super::{Decoder, Encoder, PendingReply};

pub struct Client<Data, Interface>
where
    Data: Encode + ProcDecode + EnumIdEncode + Clone + PartialEq + Eq,
    Interface: Read + Write,
{
    pending: HashMap<u32, PendingReply>,
    encoder: Encoder<Data>,
    decoder: Decoder<Data>,
    interface: Interface,
}

impl<Data, Interface> Client<Data, Interface>
where
    Data: Encode + ProcDecode + EnumIdEncode + Clone + PartialEq + Eq,
    Interface: Read + Write,
{
    pub fn new<
        const PROGRAM: u32,
        const VERSION: u32,
        D: Encode + ProcDecode + EnumIdEncode + Clone + PartialEq + Eq,
        I: Read + Write,
    >(
        interface: I,
    ) -> Client<D, I> {
        Client::<D, I> {
            pending: HashMap::new(),
            encoder: Encoder::<D>::new::<PROGRAM, VERSION, D>(),
            decoder: Decoder::<D>::new::<PROGRAM, VERSION, D>(),
            interface,
        }
    }

    pub fn send(&mut self, data: &Data) -> Result<()>
    {
        let pending = self.encoder.call(&mut self.interface, data)?;
        self.pending.insert(pending.xid, pending);
        Ok(())
    }

    pub fn recv(&mut self) -> Result<Data> {
        let res = self.decoder.
    }
}
