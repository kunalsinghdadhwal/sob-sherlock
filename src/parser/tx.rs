use std::io::{self, Cursor, Read, Seek};

use crate::{
    hash::{sha256d, to_display_hex},
    parser::varint::{read_bytes, read_compact_size, read_i32_le, read_i64_le, read_u32_le},
};

#[derive(Debug, Clone)]
pub struct RawTx {
    pub version: i32,
    pub is_segwit: bool,
    pub inputs: Vec<TxInput>,
    pub outputs: Vec<TxOutput>,
    pub locktime: u32,
    pub non_witness_bytes: Vec<u8>,

    pub txid: String,
    pub txid_hash: [u8; 32],
    pub wtxid: Option<String>,
    pub size: usize,
    pub weight: usize,
}

#[derive(Debug, Clone)]
pub struct TxInput {
    pub prev_txid: [u8; 32],
    pub vout: u32,
    pub script_sig: Vec<u8>,
    pub sequence: u32,
    pub witness: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
pub struct TxOutput {
    pub value: i64,
    pub script_pubkey: Vec<u8>,
}

impl TxInput {
    pub fn prev_txid_hex(&self) -> String {
        let mut rev = self.prev_txid;
        rev.reverse();
        hex::encode(rev)
    }
}

pub fn parse_raw_tx_bytes(raw: &[u8]) -> io::Result<RawTx> {
    let c = &mut Cursor::new(raw);

    let version = read_i32_le(c)?;

    let pos_after_version = c.position();
    let mut peek = [0u8; 1];
    Read::read_exact(c, &mut peek)?;

    let is_segwit = if peek[0] == 0x00 {
        let mut flag = [0u8; 1];
        Read::read_exact(c, &mut flag)?;

        if flag[0] != 0x01 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid Segwit Flag",
            ));
        }
        true
    } else {
        c.seek(io::SeekFrom::Start(pos_after_version))?;
        false
    };

    let inp_cnt = read_compact_size(c)? as usize;
    let mut inputs: Vec<TxInput> = Vec::with_capacity(inp_cnt);
    for _ in 0..inp_cnt {
        let mut prev_txid = [0u8; 32];
        Read::read_exact(c, &mut prev_txid)?;

        let vout = read_u32_le(c)?;
        let sig_len = read_compact_size(c)? as usize;
        let script_sig = read_bytes(c, sig_len)?;
        let sequence = read_u32_le(c)?;
        inputs.push(TxInput {
            prev_txid,
            vout,
            script_sig,
            sequence,
            witness: Vec::new(),
        });
    }

    let out_cnt = read_compact_size(c)? as usize;
    let mut outputs: Vec<TxOutput> = Vec::with_capacity(out_cnt);
    for _ in 0..out_cnt {
        let val = read_i64_le(c)?;
        let pk_len = read_compact_size(c)? as usize;
        let script_pubkey = read_bytes(c, pk_len)?;

        outputs.push(TxOutput {
            value: val,
            script_pubkey,
        });
    }

    if is_segwit {
        for input in &mut inputs {
            let item_cnt = read_compact_size(c)? as usize;
            let mut items = Vec::with_capacity(item_cnt);
            for _ in 0..item_cnt {
                let item_len = read_compact_size(c)? as usize;
                let item = read_bytes(c, item_len)?;
                items.push(item);
            }
            input.witness = items;
        }
    }

    let locktime = read_u32_le(c)?;

    let estimated = 4 + 9 + inputs.iter().map(|i| 41 + i.script_sig.len()).sum::<usize>()
        + 9 + outputs.iter().map(|o| 9 + o.script_pubkey.len()).sum::<usize>() + 4;
    let mut non_witness = Vec::with_capacity(estimated);
    non_witness.extend_from_slice(&version.to_le_bytes());

    encode_compact_size(&mut non_witness, inputs.len() as u64);
    for inp in &inputs {
        non_witness.extend_from_slice(&inp.prev_txid);
        non_witness.extend_from_slice(&inp.vout.to_le_bytes());
        encode_compact_size(&mut non_witness, inp.script_sig.len() as u64);
        non_witness.extend_from_slice(&inp.script_sig);
        non_witness.extend_from_slice(&inp.sequence.to_le_bytes());
    }

    encode_compact_size(&mut non_witness, outputs.len() as u64);

    for out in &outputs {
        non_witness.extend_from_slice(&out.value.to_le_bytes());
        encode_compact_size(&mut non_witness, out.script_pubkey.len() as u64);
        non_witness.extend_from_slice(&out.script_pubkey);
    }

    non_witness.extend_from_slice(&locktime.to_le_bytes());

    let txid_hash = sha256d(&non_witness);
    let txid = to_display_hex(&txid_hash);

    let size = c.position() as usize;
    let tx_bytes = &raw[..size];

    let wtxid = if is_segwit {
        let wtxid_hash = sha256d(tx_bytes);
        Some(to_display_hex(&wtxid_hash))
    } else {
        None
    };

    let non_witness_size = non_witness.len();
    let witness_size = size - non_witness_size;
    let weight = non_witness_size * 4 + witness_size;

    Ok(RawTx {
        version,
        is_segwit,
        inputs,
        outputs,
        locktime,
        non_witness_bytes: non_witness,
        txid,
        txid_hash,
        wtxid,
        size,
        weight,
    })
}

fn encode_compact_size(buf: &mut Vec<u8>, val: u64) {
    if val < 0xFD {
        buf.push(val as u8);
    } else if val <= 0xFFFF {
        buf.push(0xFD);
        buf.extend_from_slice(&(val as u16).to_le_bytes());
    } else if val <= 0xFFFF_FFFF {
        buf.push(0xFE);
        buf.extend_from_slice(&(val as u32).to_le_bytes());
    } else {
        buf.push(0xFF);
        buf.extend_from_slice(&val.to_le_bytes());
    }
}
