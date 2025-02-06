use crate::deserialize_in_order;
use crate::part::{V5NameSt, V8NameSt, XptHeader};

// 基础trait
pub trait FromBytes {
    fn from_bytes(input: &[u8]) -> Self;
}

// 基础类型实现
impl FromBytes for String {
    fn from_bytes(input: &[u8]) -> Self {
        String::from_utf8(input.to_vec())
            .unwrap()
            // .trim()
            .to_string()
    }
}

impl FromBytes for u32 {
    fn from_bytes(input: &[u8]) -> Self {
        u32::from_be_bytes(input.try_into().unwrap())
    }
}

impl FromBytes for u16 {
    fn from_bytes(input: &[u8]) -> Self {
        u16::from_be_bytes(input.try_into().unwrap())
    }
}

impl<const COUNT: usize> FromBytes for U8Array<COUNT> {
    fn from_bytes(input: &[u8]) -> Self {
        U8Array {
            inner: input[0..COUNT].try_into().unwrap(),
        }
    }
}
impl<T: XptHeader> FromBytes for BufferFromByteArray<T> {
    fn from_bytes(input: &[u8]) -> Self {
        BufferFromByteArray(T::new(input.try_into().unwrap()).unwrap())
    }
}

impl FromBytes for V5NameSt {
    fn from_bytes(input: &[u8]) -> Self {
        deserialize_in_order!(&input,{
            ntype:u16 with 2,
            nhfun:u16 with 2,
            nlng:u16 with 2,
            nvar0:u16 with 2,
            nname:U8Array<8> with 8,
            nlabel:U8Array<40> with 40,
            nform:U8Array<8> with 8,
            nfl:u16 with 2,
            nfd:u16 with 2,
            nfj:u16 with 2,
            nfill:U8Array<2> with 2,
            niform:U8Array<8> with 8,
            nifl:u16 with 2,
            nifd:u16 with 2,
            npos:u32 with 4
            // rest:String with 52
        });
        V5NameSt {
            ntype,
            nhfun,
            nlng,
            nvar0,
            nname,
            nlabel,
            nform,
            nfl,
            nfd,
            nfj,
            nfill: nfill.inner,
            niform,
            nifl,
            nifd,
            npos,
            rest: String::new(),
        }
    }
}

impl FromBytes for V8NameSt {
    fn from_bytes(input: &[u8]) -> Self {
        deserialize_in_order!(&input,{
            ntype:u16 with 2,
            nhfun:u16 with 2,
            nlng:u16 with 2,
            nvar0:u16 with 2,
            nname:U8Array<8> with 8,
            nlabel:U8Array<40> with 40,
            nform:U8Array<8> with 8,
            nfl:u16 with 2,
            nfd:u16 with 2,
            nfj:u16 with 2,
            nfill:U8Array<2> with 2,
            niform:U8Array<8> with 8,
            nifl:u16 with 2,
            nifd:u16 with 2,
            npos:u32 with 4,
            nlname:U8Array<32> with 32,
            lablen:u16 with 2
            // rest:String with 52
        });
        V8NameSt {
            ntype,
            nhfun,
            nlng,
            nvar0,
            nname,
            nlabel,
            nform,
            nfl,
            nfd,
            nfj,
            nfill: nfill.inner,
            niform,
            nifl,
            nifd,
            npos,
            nlname,
            lablen,
            rest: String::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct U8Array<const COUNT: usize> {
    pub(crate) inner: [u8; COUNT],
}

pub struct BufferFromByteArray<T>(pub T);
