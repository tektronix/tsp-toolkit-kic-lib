use byteorder::{BigEndian, ReadBytesExt};

use super::{sunrpc::Opaque, Decode, Encode, EnumIdEncode, ProcDecode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateLink {
    pub client_id: i32,
    pub lock_device: bool, //convert to u32, 0 == false
    pub lock_timeout: LockTimeout,
    pub device_name: String,
}

impl Encode for CreateLink {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.client_id.to_be_bytes());
        message.extend_from_slice(&u32::from(self.lock_device).to_be_bytes());
        message.extend_from_slice(&self.lock_timeout.to_encoded_bytes());
        message.extend_from_slice(&Opaque::from(self.device_name.clone()).to_encoded_bytes());
        message
    }
}

impl Decode for CreateLink {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let client_id = reader.read_i32::<BigEndian>()?;
        let lock_device = reader.read_u32::<BigEndian>()? != 0;
        let lock_timeout = LockTimeout::decode(reader)?;
        let device_name = Opaque::decode(reader)?.as_string();

        Ok(Self {
            client_id,
            lock_device,
            lock_timeout,
            device_name,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceWrite {
    pub link_id: DeviceLink,
    pub io_timeout: IoTimeout,
    pub lock_timeout: LockTimeout,
    pub flags: DeviceFlags,
    pub data: Vec<u8>,
}

impl Encode for DeviceWrite {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.link_id.to_encoded_bytes());
        message.extend_from_slice(&self.io_timeout.to_encoded_bytes());
        message.extend_from_slice(&self.lock_timeout.to_encoded_bytes());
        message.extend_from_slice(&self.flags.to_encoded_bytes());
        message.extend(&Opaque::from(self.data.clone()).to_encoded_bytes());

        message
    }
}

impl Decode for DeviceWrite {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let link_id = DeviceLink::decode(reader)?;
        let io_timeout = IoTimeout::decode(reader)?;
        let lock_timeout = LockTimeout::decode(reader)?;
        let flags = DeviceFlags::decode(reader)?;
        let data = Opaque::decode(reader)?.as_bytes().to_vec();

        Ok(Self {
            link_id,
            io_timeout,
            lock_timeout,
            flags,
            data,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceRead {
    pub link_id: DeviceLink,
    pub request_size: u32,
    pub io_timeout: IoTimeout,
    pub lock_timeout: LockTimeout,
    pub flags: DeviceFlags,
    pub term_char: u8,
}

impl Encode for DeviceRead {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.link_id.to_encoded_bytes());
        message.extend_from_slice(&self.request_size.to_be_bytes());
        message.extend_from_slice(&self.io_timeout.to_encoded_bytes());
        message.extend_from_slice(&self.lock_timeout.to_encoded_bytes());
        message.extend_from_slice(&self.flags.to_encoded_bytes());
        message.extend_from_slice(&u32::from(self.term_char).to_be_bytes());

        message
    }
}

impl Decode for DeviceRead {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let link_id: DeviceLink = DeviceLink::decode(reader)?;
        let request_size: u32 = reader.read_u32::<BigEndian>()?;
        let io_timeout: IoTimeout = IoTimeout::decode(reader)?;
        let lock_timeout: LockTimeout = LockTimeout::decode(reader)?;
        let flags: DeviceFlags = DeviceFlags::decode(reader)?;
        let term_char: u8 = u8::try_from(reader.read_u32::<BigEndian>()?)?;

        Ok(Self {
            link_id,
            request_size,
            io_timeout,
            lock_timeout,
            flags,
            term_char,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    pub link_id: DeviceLink,
    pub flags: DeviceFlags,
    pub lock_timeout: LockTimeout,
}

impl Encode for Lock {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.link_id.to_encoded_bytes());
        message.extend_from_slice(&self.flags.to_encoded_bytes());
        message.extend_from_slice(&self.lock_timeout.to_encoded_bytes());

        message
    }
}

impl Decode for Lock {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let link_id = DeviceLink::decode(reader)?;
        let flags = DeviceFlags::decode(reader)?;
        let lock_timeout = LockTimeout::decode(reader)?;
        Ok(Self {
            link_id,
            flags,
            lock_timeout,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateInterruptChannel {
    pub host_address: u32,
    pub host_port: u16,
    pub program_number: u32,
    pub program_version: u32,
    pub program_family: AddressFamily,
}

impl Encode for CreateInterruptChannel {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.host_address.to_be_bytes());
        message.extend_from_slice(&u32::from(self.host_port).to_be_bytes());
        message.extend_from_slice(&self.program_number.to_be_bytes());
        message.extend_from_slice(&self.program_version.to_be_bytes());
        message.extend_from_slice(&self.program_family.to_encoded_bytes());

        message
    }
}

impl Decode for CreateInterruptChannel {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let host_address = reader.read_u32::<BigEndian>()?;
        let host_port = u16::try_from(reader.read_u32::<BigEndian>()?)?;
        let program_number = reader.read_u32::<BigEndian>()?;
        let program_version = reader.read_u32::<BigEndian>()?;
        let program_family = match reader.read_u32::<BigEndian>() {
            Ok(x) if x == AddressFamily::Tcp as u32 => AddressFamily::Tcp,
            Ok(x) if x == AddressFamily::Udp as u32 => AddressFamily::Udp,
            Ok(e) => {
                return Err(super::Error::DecodeError(format!(
                    "unrecognized address family {e}"
                )))
            }
            Err(e) => return Err(e.into()),
        };

        Ok(Self {
            host_address,
            host_port,
            program_number,
            program_version,
            program_family,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnableSrq {
    pub link_id: DeviceLink,
    pub enable: bool,
    pub handle: [u8; 40],
}

impl Encode for EnableSrq {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.link_id.to_encoded_bytes());
        message.extend_from_slice(&u32::from(self.enable).to_be_bytes());
        message.extend_from_slice(&self.handle);

        message
    }
}

impl Decode for EnableSrq {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let link_id = DeviceLink::decode(reader)?;
        let enable = reader.read_u32::<BigEndian>()? != 0;

        let mut handle: [u8; 40] = [0u8; 40];
        reader.read_exact(&mut handle)?;

        Ok(Self {
            link_id,
            enable,
            handle,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoCommand {
    pub link_id: DeviceLink,
    pub flags: DeviceFlags,
    pub io_timeout: IoTimeout,
    pub lock_timeout: LockTimeout,
    pub command: i32,
    pub network_order: bool,
    pub datasize: i32,
    pub data_in: Vec<u8>,
}

impl Encode for DoCommand {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.link_id.to_encoded_bytes());
        message.extend_from_slice(&self.flags.to_encoded_bytes());
        message.extend_from_slice(&self.io_timeout.to_encoded_bytes());
        message.extend_from_slice(&self.lock_timeout.to_encoded_bytes());
        message.extend_from_slice(&self.command.to_be_bytes());
        message.extend_from_slice(&u32::from(self.network_order).to_be_bytes());
        message.extend_from_slice(&self.datasize.to_be_bytes());
        message.extend_from_slice(&Opaque::from(self.data_in.clone()).to_encoded_bytes());

        message
    }
}

impl Decode for DoCommand {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let link_id = DeviceLink::decode(reader)?;
        let flags = DeviceFlags::decode(reader)?;
        let io_timeout = IoTimeout::decode(reader)?;
        let lock_timeout = LockTimeout::decode(reader)?;
        let command = reader.read_i32::<BigEndian>()?;
        let network_order = reader.read_u32::<BigEndian>()? != 0;
        let datasize = reader.read_i32::<BigEndian>()?;
        let data_in = Opaque::decode(reader)?.as_bytes().to_vec();

        Ok(Self {
            link_id,
            flags,
            io_timeout,
            lock_timeout,
            command,
            network_order,
            datasize,
            data_in,
        })
    }
}

pub struct InterruptSrq {
    pub handle: Vec<u8>,
}

impl Encode for InterruptSrq {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        Opaque::from(self.handle.clone()).to_encoded_bytes()
    }
}

impl Decode for InterruptSrq {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let handle = Opaque::decode(reader)?.as_bytes().to_vec();
        Ok(Self { handle })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum CoreRequest {
    CreateLink(CreateLink) = 10,
    Write(DeviceWrite) = 11,
    DeviceRead(DeviceRead) = 12,
    ReadStb(GenericParams) = 13,
    Trigger(GenericParams) = 14,
    Clear(GenericParams) = 15,
    Remote(GenericParams) = 16,
    Local(GenericParams) = 17,
    Lock(Lock) = 18,
    Unlock(DeviceLink) = 19,
    EnableSrq(EnableSrq) = 20,
    DoCommand(DoCommand) = 22,
    DestroyLink(DeviceLink) = 23,
    CreateInterruptChannel(CreateInterruptChannel) = 25,
    DestroyInterruptChannel = 26,
}

impl EnumIdEncode for CoreRequest {}

impl Encode for CoreRequest {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        match self {
            Self::CreateLink(x) => x.to_encoded_bytes(),
            Self::Write(x) => x.to_encoded_bytes(),
            Self::DeviceRead(x) => x.to_encoded_bytes(),
            Self::ReadStb(x)
            | Self::Trigger(x)
            | Self::Clear(x)
            | Self::Remote(x)
            | Self::Local(x) => x.to_encoded_bytes(),
            Self::Lock(x) => x.to_encoded_bytes(),
            Self::Unlock(x) | Self::DestroyLink(x) => x.to_encoded_bytes(),
            Self::EnableSrq(x) => x.to_encoded_bytes(),
            Self::DoCommand(x) => x.to_encoded_bytes(),
            Self::CreateInterruptChannel(x) => x.to_encoded_bytes(),
            Self::DestroyInterruptChannel => Vec::new(),
        }
    }
}

impl ProcDecode for CoreRequest {
    fn proc_decode<R: std::io::Read>(reader: &mut R, procedure_id: u32) -> super::Result<Self> {
        Ok(match procedure_id {
            10 => Self::CreateLink(CreateLink::decode(reader)?),
            11 => Self::Write(DeviceWrite::decode(reader)?),
            12 => Self::DeviceRead(DeviceRead::decode(reader)?),
            13 => Self::ReadStb(GenericParams::decode(reader)?),
            14 => Self::Trigger(GenericParams::decode(reader)?),
            15 => Self::Clear(GenericParams::decode(reader)?),
            16 => Self::Remote(GenericParams::decode(reader)?),
            17 => Self::Local(GenericParams::decode(reader)?),
            18 => Self::Lock(Lock::decode(reader)?),
            19 => Self::Unlock(DeviceLink::decode(reader)?),
            20 => Self::EnableSrq(EnableSrq::decode(reader)?),
            22 => Self::DoCommand(DoCommand::decode(reader)?),
            23 => Self::DestroyLink(DeviceLink::decode(reader)?),
            25 => Self::CreateInterruptChannel(CreateInterruptChannel::decode(reader)?),
            26 => Self::DestroyInterruptChannel,
            _ => {
                return Err(super::Error::DecodeError(format!(
                    "unrecognized procedure number {procedure_id}"
                )))
            }
        })
    }
}

#[repr(u32)]
pub enum AbortRequest {
    Abort(DeviceLink) = 1,
}

#[repr(u32)]
pub enum InterruptRequest {
    InterruptSrq(InterruptSrq) = 30,
}

#[repr(u32)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressFamily {
    Tcp = 0u32,
    Udp = 1u32,
}

impl Encode for AddressFamily {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        match self {
            Self::Tcp => Self::Tcp as u32,
            Self::Udp => Self::Udp as u32,
        }
        .to_be_bytes()
        .to_vec()
    }
}

pub struct CreateLinkReply {
    pub error: DeviceErrorCode,
    pub link_id: DeviceLink,
    pub abort_port: u16,
    pub max_rcv_size: u32,
}

pub struct WriteReply {
    pub error: DeviceErrorCode,
    pub size: u32,
}

pub struct ReadReply {
    pub error: DeviceErrorCode,
    pub reason: ReadCompletionReason,
}

pub struct ReadStbReply {
    pub error: DeviceErrorCode,
    pub stb: u8,
}

pub struct DoCommandReply {
    pub error: DeviceErrorCode,
    pub data_out: Vec<u8>,
}

#[repr(u32)]
pub enum CoreResponse {
    CreateLinkReply(CreateLinkReply) = 10,
    WriteReply(WriteReply) = 11,
    ReadReply(ReadReply) = 12,
    ReadStbReply(ReadStbReply) = 13,
    TriggerReply(DeviceError) = 14,
    ClearReply(DeviceError) = 15,
    RemoteReply(DeviceError) = 16,
    LocalReply(DeviceError) = 17,
    LockReply(DeviceError) = 18,
    UnlockReply(DeviceError) = 19,
    EnableSrqReply(DeviceError) = 20,
    DoCommandReply(DoCommandReply) = 22,
    DestroyLinkReply(DeviceError) = 23,
    CreateInterruptChannelReply(DeviceError) = 25,
    DestroyInterruptChannelReply(DeviceError) = 26,
}

#[repr(u32)]
pub enum InterruptResponse {
    InterruptSrqReply = 30,
}

#[repr(u32)]
pub enum AbortResponse {
    AbortReply(DeviceError) = 1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceLink(u32);

impl Encode for DeviceLink {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
}

impl Decode for DeviceLink {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        Ok(Self(reader.read_u32::<BigEndian>()?))
    }
}

#[repr(u32)]
pub enum DeviceErrorCode {
    None = 0,
    Syntax = 1,
    DeviceNotAccessible = 3,
    InvalidLinkIdentifier = 4,
    ParameterError = 5,
    ChannelNotEstablished = 6,
    OperationNotSupported = 8,
    OutOfResources = 9,
    DeviceLockedByAnotherLink = 11,
    NoLockHeldByThisLink = 12,
    IoTimeout = 15,
    IoError = 17,
    InvalidAddress = 21,
    Abort = 23,
    ChannelAlreadyEstablished = 29,
}

pub struct DeviceError {
    pub error: DeviceErrorCode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceFlags {
    flags: u32,
}

impl Encode for DeviceFlags {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        self.flags.to_be_bytes().to_vec()
    }
}

impl Decode for DeviceFlags {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        Ok(Self {
            flags: reader.read_u32::<BigEndian>()?,
        })
    }
}

const fn is_bit_set(x: u32, bit: u8) -> bool {
    ((x >> bit) & 0b1) == 1
}

const fn set_bit(value: u32, bit: u8, enable: bool) -> u32 {
    if enable {
        value | (1 << bit)
    } else {
        value & !(1 << bit)
    }
}

impl DeviceFlags {
    pub fn waitlock(&mut self, enable: bool) -> &Self {
        self.flags = set_bit(self.flags, 0, enable);
        self
    }

    pub fn end(&mut self, enable: bool) -> &Self {
        self.flags = set_bit(self.flags, 3, enable);
        self
    }

    pub fn termchrset(&mut self, enable: bool) -> &Self {
        self.flags = set_bit(self.flags, 7, enable);
        self
    }

    #[must_use]
    pub const fn is_waitlock(&self) -> bool {
        is_bit_set(self.flags, 0)
    }

    #[must_use]
    pub const fn is_end(&self) -> bool {
        is_bit_set(self.flags, 3)
    }

    #[must_use]
    pub const fn is_termchrset(&self) -> bool {
        is_bit_set(self.flags, 7)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParams {
    lid: DeviceLink,
    flags: DeviceFlags,
    lock_timeout: LockTimeout,
    io_timeout: IoTimeout,
}

impl Encode for GenericParams {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.lid.to_encoded_bytes());
        message.extend_from_slice(&self.flags.to_encoded_bytes());
        message.extend_from_slice(&self.lock_timeout.to_encoded_bytes());
        message.extend_from_slice(&self.io_timeout.to_encoded_bytes());

        message
    }
}

impl Decode for GenericParams {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        let lid = DeviceLink::decode(reader)?;
        let flags = DeviceFlags::decode(reader)?;
        let lock_timeout = LockTimeout::decode(reader)?;
        let io_timeout = IoTimeout::decode(reader)?;

        Ok(Self {
            lid,
            flags,
            lock_timeout,
            io_timeout,
        })
    }
}

/// In ms
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoTimeout(u32);

impl Encode for IoTimeout {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
}

impl Decode for IoTimeout {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        Ok(Self(reader.read_u32::<BigEndian>()?))
    }
}

/// In ms
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockTimeout(u32);

impl Encode for LockTimeout {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
}

impl Decode for LockTimeout {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        Ok(Self(reader.read_u32::<BigEndian>()?))
    }
}

pub struct ReadCompletionReason {
    reason: u32,
}

impl ReadCompletionReason {
    pub fn end(&mut self, enable: bool) -> &Self {
        self.reason = set_bit(self.reason, 2, enable);
        self
    }

    pub fn term_char_sent(&mut self, enable: bool) -> &Self {
        self.reason = set_bit(self.reason, 1, enable);
        self
    }

    pub fn requested_size_sent(&mut self, enable: bool) -> &Self {
        self.reason = set_bit(self.reason, 0, enable);
        self
    }

    #[must_use]
    pub const fn is_end(&self) -> bool {
        is_bit_set(self.reason, 2)
    }

    #[must_use]
    pub const fn is_term_char_sent(&self) -> bool {
        is_bit_set(self.reason, 1)
    }

    #[must_use]
    pub const fn is_requested_size_sent(&self) -> bool {
        is_bit_set(self.reason, 0)
    }
}
