use byteorder::{BigEndian, ReadBytesExt};

use crate::vxi11::{Decode, Encode};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opaque {
    data: Vec<u8>,
}

impl Encode for Opaque {
    fn to_encoded_bytes(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&self.data.len().to_be_bytes());
        message.extend(self.data.iter());

        //add fill bytes so the data size is a multiple of 4
        message.resize(message.len().saturating_add(self.data.len() % 4), 0u8);

        message
    }
}

impl Opaque {
    pub fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.data).to_string()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

impl From<String> for Opaque {
    fn from(value: String) -> Self {
        Self {
            data: value.as_bytes().to_vec(),
        }
    }
}

impl From<Vec<u8>> for Opaque {
    fn from(value: Vec<u8>) -> Self {
        Self { data: value }
    }
}
impl From<Vec<u16>> for Opaque {
    fn from(value: Vec<u16>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl From<Vec<i16>> for Opaque {
    fn from(value: Vec<i16>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl From<Vec<u32>> for Opaque {
    fn from(value: Vec<u32>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl From<Vec<i32>> for Opaque {
    fn from(value: Vec<i32>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl From<Vec<u64>> for Opaque {
    fn from(value: Vec<u64>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl From<Vec<i64>> for Opaque {
    fn from(value: Vec<i64>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl From<Vec<usize>> for Opaque {
    fn from(value: Vec<usize>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl From<Vec<isize>> for Opaque {
    fn from(value: Vec<isize>) -> Self {
        let mut data = Vec::new();
        for x in value {
            data.extend_from_slice(&x.to_be_bytes());
        }

        Self { data }
    }
}

impl Decode for Opaque {
    fn decode<R: std::io::Read>(reader: &mut R) -> crate::vxi11::Result<Self> {
        let len = reader.read_u32::<BigEndian>()?;

        let mut data = Vec::new();

        for _ in 0..len {
            data.push(reader.read_u8()?);
        }

        Ok(Self { data })
    }
}
