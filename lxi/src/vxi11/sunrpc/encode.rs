use crate::vxi11::{Encode, EnumIdEncode, ProcDecode};

use super::{AuthFlavor, Header, OpaqueAuth, PendingReply, ReplyBody};

pub struct Encoder<Data: Encode + ProcDecode + EnumIdEncode + PartialEq + Eq + Clone> {
    xid: u32,
    data_type: std::marker::PhantomData<Data>,
    program: u32,
    version: u32,
}

impl<Data: Encode + ProcDecode + EnumIdEncode + Clone + PartialEq + Eq> Encoder<Data> {
    pub const fn new<
        const PROGRAM: u32,
        const VERSION: u32,
        D: Encode + ProcDecode + EnumIdEncode + PartialEq + Eq + Clone,
    >() -> Encoder<D> {
        Encoder::<D> {
            xid: 0,
            data_type: std::marker::PhantomData::<D>,
            program: PROGRAM,
            version: VERSION,
        }
    }

    pub fn call<W: std::io::Write>(
        &mut self,
        writer: &mut W,
        data: &Data,
    ) -> std::io::Result<PendingReply> {
        self.xid = self.xid.wrapping_add(1);
        let msg = Header::call(
            self.program,
            self.version,
            self.xid,
            data.clone(),
            OpaqueAuth {
                flavor: AuthFlavor::None,
            },
            OpaqueAuth {
                flavor: AuthFlavor::None,
            },
        );
        msg.encode(writer)?;
        Ok(PendingReply {
            xid: self.xid,
            procedure: data.variant_id(),
            program: self.program,
            version: self.version,
        })
    }

    pub fn reply<W: std::io::Write>(
        &mut self,
        writer: &mut W,
        body: ReplyBody<Data>,
    ) -> std::io::Result<()> {
        self.xid = self.xid.wrapping_add(1);
        let msg = Header::reply(self.xid, body);
        msg.encode(writer)?;
        Ok(())
    }
}

