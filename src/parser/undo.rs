use std::io::{self, Cursor, Read};

use crate::parser::varint::read_compact_size;

#[derive(Debug, Clone)]
pub struct UndoCoin {
    pub height: u32,
    pub coinbase: bool,
    pub amount: u64,
    pub script_pubkey: Vec<u8>,
}

#[derive(Debug)]
pub struct TxUndo {
    pub prevouts: Vec<UndoCoin>,
}

#[derive(Debug)]
pub struct BlockUndo {
    pub tx_undos: Vec<TxUndo>,
}

pub fn parse_rev_file(data: &[u8], num_blocks: usize) -> io::Result<Vec<BlockUndo>> {
    let mut undos = Vec::new();
    let mut pos: usize = 0;

    for _ in 0..num_blocks {
        if pos + 8 > data.len() {
            break;
        }

        let magic = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        pos += 4;
        if magic != 0xD9B4BEF9 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Bad rev magic"));
        }
        let block_size =
            u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;

        let undo_data = &data[pos..pos + block_size];
        let mut cursor = Cursor::new(undo_data);

        let tx_count = read_compact_size(&mut cursor)? as usize;
        let mut tx_undos = Vec::with_capacity(tx_count);

        for _ in 0..tx_count {
            let inp_count = read_compact_size(&mut cursor)? as usize;
            let mut prevouts = Vec::with_capacity(inp_count);
            for _ in 0..inp_count {
                let coin = read_undo_coin(&mut cursor)?;
                prevouts.push(coin);
            }
            tx_undos.push(TxUndo { prevouts });
        }

        undos.push(BlockUndo { tx_undos });

        pos += block_size + 32;
    }

    Ok(undos)
}

fn read_undo_coin(cursor: &mut Cursor<&[u8]>) -> io::Result<UndoCoin> {
    let n_code = read_bitcoin_varint(cursor)?;
    let height = (n_code / 2) as u32;
    let coinbase = (n_code & 1) != 0;

    if height > 0 {
        let mut dummy = [0u8; 1];
        cursor.read_exact(&mut dummy)?;
    }

    let amount = decompress_amount(read_bitcoin_varint(cursor)?);

    let script_pubkey = read_compressed_script(cursor)?;

    Ok(UndoCoin {
        height,
        coinbase,
        amount,
        script_pubkey,
    })
}

fn read_bitcoin_varint(cursor: &mut Cursor<&[u8]>) -> io::Result<u64> {
    let mut n: u64 = 0;
    loop {
        let mut buf = [0u8; 1];
        cursor.read_exact(&mut buf)?;
        let ch = buf[0];
        if ch < 128 {
            n = n.wrapping_mul(128).wrapping_add(ch as u64);
            break;
        } else {
            n = n.wrapping_mul(128).wrapping_add((ch & 0x7F) as u64 + 1);
        }
    }
    Ok(n)
}

fn decompress_amount(x: u64) -> u64 {
    if x == 0 {
        return 0;
    }
    let mut x = x - 1;
    let e = (x % 10) as u32;
    x /= 10;
    let n: u64;
    if e < 9 {
        let d = (x % 9) + 1;
        x /= 9;
        n = x * 10 + d;
    } else {
        n = x + 1;
    }
    n * 10u64.pow(e)
}

fn read_compressed_script(cursor: &mut Cursor<&[u8]>) -> io::Result<Vec<u8>> {
    let n_size = read_bitcoin_varint(cursor)? as usize;

    match n_size {
        0 => {
            let mut hash = [0u8; 20];
            cursor.read_exact(&mut hash)?;
            let mut script = Vec::with_capacity(25);
            script.push(0x76); // OP_DUP
            script.push(0xa9); // OP_HASH160
            script.push(0x14); // OP_PUSHBYTES_20
            script.extend_from_slice(&hash);
            script.push(0x88); // OP_EQUALVERIFY
            script.push(0xac); // OP_CHECKSIG
            Ok(script)
        }
        1 => {
            let mut hash = [0u8; 20];
            cursor.read_exact(&mut hash)?;
            let mut script = Vec::with_capacity(23);
            script.push(0xa9); // OP_HASH160
            script.push(0x14); // OP_PUSHBYTES_20
            script.extend_from_slice(&hash);
            script.push(0x87); // OP_EQUAL
            Ok(script)
        }
        2 | 3 => {
            let mut key = [0u8; 32];
            cursor.read_exact(&mut key)?;
            let mut script = Vec::with_capacity(35);
            script.push(0x21); // OP_PUSHBYTES_33
            script.push(n_size as u8);
            script.extend_from_slice(&key);
            script.push(0xac); // OP_CHECKSIG
            Ok(script)
        }
        4 | 5 => {
            let mut key = [0u8; 32];
            cursor.read_exact(&mut key)?;
            let prefix = if n_size == 4 { 0x02 } else { 0x03 };
            let mut script = Vec::with_capacity(35);
            script.push(0x21); // OP_PUSHBYTES_33
            script.push(prefix);
            script.extend_from_slice(&key);
            script.push(0xac); // OP_CHECKSIG
            Ok(script)
        }
        _ => {
            let script_len = n_size - 6;
            let mut script = vec![0u8; script_len];
            cursor.read_exact(&mut script)?;
            Ok(script)
        }
    }
}
