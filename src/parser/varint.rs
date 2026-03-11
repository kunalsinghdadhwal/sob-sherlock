use std::io::{self, Cursor, Read};

pub fn read_compact_size(c: &mut Cursor<&[u8]>) -> io::Result<u64> {
    let mut first = [0u8; 1];

    c.read_exact(&mut first)?;
    match first[0] {
        0x00..=0xFC => Ok(first[0] as u64),
        0xFD => {
            let mut buf = [0u8; 2];
            c.read_exact(&mut buf)?;
            Ok(u16::from_le_bytes(buf) as u64)
        }
        0xFE => {
            let mut buf = [0u8; 4];
            c.read_exact(&mut buf)?;
            Ok(u32::from_le_bytes(buf) as u64)
        }
        0xFF => {
            let mut buf = [0u8; 8];
            c.read_exact(&mut buf)?;
            Ok(u64::from_le_bytes(buf))
        }
    }
}

pub fn read_bytes(c: &mut Cursor<&[u8]>, n: usize) -> io::Result<Vec<u8>> {
    let mut buf = vec![0u8; n];
    c.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn read_u32_le(c: &mut Cursor<&[u8]>) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    c.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

pub fn read_i32_le(c: &mut Cursor<&[u8]>) -> io::Result<i32> {
    let mut buf = [0u8; 4];
    c.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

pub fn read_i64_le(c: &mut Cursor<&[u8]>) -> io::Result<i64> {
    let mut buf = [0u8; 8];
    c.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}
