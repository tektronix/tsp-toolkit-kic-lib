use super::{sunrpc::{self, PendingReply, ReplyBody}, Decode, Encode, EnumIdEncode};

pub struct CreateLink{
    pub client_id: i32,
    pub lock_device: bool,//convert to u32, 0 == false
    pub lock_timeout: LockTimeout,
    pub device_name: String,
}

impl Encode for CreateLink {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for CreateLink {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

pub struct DeviceWrite{
    pub link_id: DeviceLink,
    pub io_timeout: IoTimeout,
    pub lock_timeout: LockTimeout,
    pub flags: DeviceFlags,
    pub data: Vec<u8>,
}

impl Encode for DeviceWrite {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for DeviceWrite {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

pub struct DeviceRead{
    pub link_id: DeviceLink,
    pub request_size: u32,
    pub io_timeout: IoTimeout,
    pub lock_timeout: LockTimeout,
    pub flags: DeviceFlags,
    pub term_char: u8,
}

impl Encode for DeviceRead {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for DeviceRead {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

pub struct Lock{
    pub link_id: DeviceLink,
    pub flags: DeviceFlags,
    pub lock_timeout: LockTimeout,
}

impl Encode for Lock {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for Lock {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

pub struct CreateInterruptChannel{
    pub host_address: u32,
    pub host_port: u16,
    pub program_number: u32,
    pub program_version: u32,
    pub program_family: AddressFamily,
}

impl Encode for CreateInterruptChannel {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for CreateInterruptChannel {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

pub struct EnableSrq{
    pub link_id: DeviceLink,
    pub enable: bool,
    pub handle: [u8; 40],
}

impl Encode for EnableSrq {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for EnableSrq {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

pub struct DoCommand{
    pub link_id: DeviceLink,
    pub flags: DeviceFlags,
    pub io_timeout: IoTimeout,
    pub lock_timeout: LockTimeout,
    pub command: i32,
    pub network_order: bool,
    pub datasize: i32,
    pub data_in: Vec<u8>
}

impl Encode for DoCommand {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for DoCommand {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

pub struct InterruptSrq{
    pub handle: Vec<u8>,
}

impl Encode for InterruptSrq {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for InterruptSrq {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
    }
}

#[repr(u32)]
pub enum CoreRequest {
    CreateLink(CreateLink) = 10,
    Write(DeviceWrite)= 11,
    DeviceRead(DeviceRead) = 12,
    ReadStb(GenericParams) = 13,
    Trigger(GenericParams) = 14,
    Clear(GenericParams) = 15,
    Remote(GenericParams) = 16,
    Local(GenericParams) = 17,
    Lock(Lock) = 18,
    Unlock(DeviceLink) = 19,
    EnableSrq(EnableSrq)= 20,
    DoCommand(DoCommand) = 22,
    DestroyLink(DeviceLink) = 23,
    CreateInterruptChannel(CreateInterruptChannel) = 25,
    DestroyInterruptChannel = 26,
}

impl EnumIdEncode for CoreRequest {}

impl Encode for CoreRequest {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        todo!()
    }
}

impl Decode for CoreRequest {
    fn decode<R: std::io::Read>(reader: &mut R) -> super::Result<Self> {
        todo!()
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
enum AddressFamily {
    TCP = 0,
    UDP = 1,
}

pub struct CreateLinkReply{
    pub error: DeviceErrorCode,
    pub link_id: DeviceLink,
    pub abort_port: u16,
    pub max_rcv_size: u32,
}

pub struct WriteReply{
    pub error: DeviceErrorCode,
    pub size: u32,
}

pub struct ReadReply{
    pub error: DeviceErrorCode,
    pub reason: ReadCompletionReason,
}

pub struct ReadStbReply{
    pub error: DeviceErrorCode,
    pub stb: u8,
}

pub struct DoCommandReply{
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

pub type DeviceLink = u32;

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
    error: DeviceErrorCode,
}

pub struct DeviceFlags {
    flags: u32,
}

const fn is_bit_set(x: u32, bit: u8) -> bool {
    ((x >> bit) & 0b1) == 1
}

const fn set_bit(value: u32, bit: u8, enable: bool) -> u32 {
    if enable {
        value | ( 1 << bit)
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

pub struct GenericParams {
    lid: DeviceLink,
    flags: DeviceFlags,
    lock_timeout: LockTimeout,
    io_timeout: IoTimeout,
}

/// In ms
pub type IoTimeout = u32;

/// In ms
pub type LockTimeout = u32;

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

