use bytes::{Buf, BufMut, BytesMut};
pub struct LengthPrefix {}

impl LengthPrefix {
    pub fn new() -> Self {
        Self {}
    }

    pub fn encode(item: &str) -> (BytesMut, usize) {
        let mut dst = BytesMut::new();
        dst.reserve(item.len());
        let len = item.len() as u32;
        dst.put(item.as_bytes());
        (dst, len as usize)
    }
    pub fn decode(src: &mut BytesMut) -> Option<String> {
        let s = String::from_utf8(src.to_vec()).unwrap();
        Some(s)
    }
}

impl Default for LengthPrefix {
    fn default() -> Self {
        Self::new()
    }
}