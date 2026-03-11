use std::io::{self, Cursor, Read};

use crate::hash::sha256d;
use crate::parser::tx::{self, RawTx};
use crate::parser::varint::{read_compact_size, read_i32_le, read_u32_le};

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_block_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
    pub block_hash: [u8; 32],
}

#[derive(Debug)]
pub struct ParsedBlock {
    pub header: BlockHeader,
    pub transactions: Vec<RawTx>,
}

pub fn parse_blk_file(data: &[u8]) -> io::Result<Vec<ParsedBlock>> {
    let mut blocks = Vec::new();
    let mut cursor = Cursor::new(data);

    while (cursor.position() as usize) + 8 <= data.len() {
        let magic = read_u32_le(&mut cursor)?;
        if magic != 0xD9B4BEF9 {
            break;
        }

        let block_size = read_u32_le(&mut cursor)? as usize;
        let block_start = cursor.position() as usize;

        if block_start + block_size > data.len() {
            break;
        }

        let block_data = &data[block_start..block_start + block_size];
        let block = parse_single_block(block_data)?;
        blocks.push(block);

        cursor.set_position((block_start + block_size) as u64);
    }

    Ok(blocks)
}

fn parse_single_block(data: &[u8]) -> io::Result<ParsedBlock> {
    let mut cursor = Cursor::new(data);

    let header_bytes = &data[0..80];
    let block_hash = sha256d(header_bytes);

    let version = read_i32_le(&mut cursor)?;

    let mut prev_block_hash = [0u8; 32];
    cursor.read_exact(&mut prev_block_hash)?;

    let mut merkle_root = [0u8; 32];
    cursor.read_exact(&mut merkle_root)?;

    let timestamp = read_u32_le(&mut cursor)?;
    let bits = read_u32_le(&mut cursor)?;
    let nonce = read_u32_le(&mut cursor)?;

    let header = BlockHeader {
        version,
        prev_block_hash,
        merkle_root,
        timestamp,
        bits,
        nonce,
        block_hash,
    };

    let tx_count = read_compact_size(&mut cursor)? as usize;
    let mut transactions = Vec::with_capacity(tx_count);

    for _ in 0..tx_count {
        let start = cursor.position() as usize;
        let remaining = &data[start..];
        let raw_tx = tx::parse_raw_tx_bytes(remaining)?;
        cursor.set_position((start + raw_tx.size) as u64);
        transactions.push(raw_tx);
    }

    Ok(ParsedBlock {
        header,
        transactions,
    })
}
