pub(crate) mod sunrpc;
mod error;
mod vxi11_types;

pub use vxi11_types::*;
pub use error::*;

const CORE_PROGRAM: u32 = 395_183; //VXI-11
const ABORT_PROGRAM: u32 = 395_184; //VXI-11
const INTERRUPT_PROGRAM: u32 = 395_185; //VXI-11
const VERSION: u32 = 1;

pub struct CoreEncoder {
    sunrpc_encoder: sunrpc::Encoder<CoreRequest>,
}

impl CoreEncoder {
    #[must_use]
    pub const fn new() -> CoreEncoder {
        CoreEncoder {
            sunrpc_encoder: sunrpc::Encoder::<CoreRequest>::new::<CORE_PROGRAM, VERSION, D>(),
        }
    }

    /// Write a VXI-11 Call to the given writer.
    ///
    /// # Errors
    /// Errors are those returned by [`std::io::Write`] calls.
    pub fn call<W: std::io::Write>(&mut self, writer: &mut W, data: Data) -> std::io::Result<sunrpc::PendingReply> {
        self.sunrpc_encoder.call(writer, data)
    }

    /// Write a VXI-11 Reply to the given writer.
    ///
    /// # Errors
    /// Errors are those returned by [`std::io::Write`] calls.
    pub fn reply<W: std::io::Write>(&mut self, writer: &mut W, data: sunrpc::ReplyBody<Data>) -> std::io::Result<()> {
        self.sunrpc_encoder.reply(writer, data)
    }
}

pub struct CoreDecoder;

impl CoreDecoder {
    pub fn decode<R: std::io::Read>() -> self::error::Result<()> {
        Ok(())
    }
}


pub trait EnumIdEncode {
    /// Get the descriminant of `self`. This function works for all enum-types
    /// with a [`u32`] representation
    ///
    /// # Safety
    /// Unsafe code is required to get the descriminant. This code converts
    /// `self` to a `*const Self` and then converts that to a `*const u32`.
    /// This is the canonical way to get the discriminant from the
    /// [enumerations pointer
    /// casting](https://doc.rust-lang.org/reference/items/enumerations.html#pointer-casting)
    /// section in the Rust Reference.
    ///
    /// # Example
    /// ```rust
    /// use lxi::vxi11::EnumIdEncode;
    /// #[repr(u32)] //this is required for this trait impl
    /// enum PayloadType<'a> {
    ///     A = 64u32, // Need to set the discriminant value, the `u32` suffix is not required.
    ///     B(u64) = 2u32,
    ///     C{ name: &'a str, id: u32 } = 14u32,
    /// }
    ///
    /// impl<'a> EnumIdEncode for PayloadType<'a> {}
    /// ```
    fn variant_id(&self) -> u32 {
        // Safety:
        // Unsafe code is required to get the descriminant. This code converts
        // `self` to a `*const Self` and then converts that to a `*const u32`.
        // This is the canonical way to get the discriminant from the
        // [enumerations pointer
        // casting](https://doc.rust-lang.org/reference/items/enumerations.html#pointer-casting)
        // section in the Rust Reference.
        unsafe { *std::ptr::from_ref::<Self>(self).cast::<u32>() }
    }
}
pub trait Encode {
    /// Encode `self` and write it to `writer`
    ///
    /// # Errors
    /// May return any errors that [`std::io::Write::write`] may produce.
    fn encode<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.to_encoded_bytes())?;
        Ok(())
    }

    /// Define how to encode `Self` (VXI-11 requires Big-Endian encoding).
    fn to_encoded_bytes(&self) -> Vec<u8>;
}

pub trait Decode
where Self: Sized,
{
    /// Decode `self` from a [`std::io::Read`]er.
    ///
    /// # Errors
    /// May return any errors that [`std::io::Read::read`] may produce as well
    /// as any conversion errors.
    fn decode<R: std::io::Read>(reader: &mut R) -> Result<Self>;
}
