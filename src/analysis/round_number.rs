use serde::Serialize;

use crate::parser::tx::RawTx;

/// Result of the round-number payment heuristic.
#[derive(Debug, Clone, Serialize)]
pub struct RoundNumberResult {
    pub detected: bool,
    /// Indices of outputs with round values.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub round_output_indices: Vec<usize>,
}

/// Detects outputs with round satoshi values (likely intentional payments).
///
/// When humans choose payment amounts, they tend to pick round numbers in
/// BTC terms: 0.1 BTC, 0.01 BTC, 1 BTC, etc. Change outputs, by contrast,
/// are whatever is left over after subtracting the payment and fee, so they
/// are almost never round.
///
/// An output value is considered "round" if it is divisible by any of:
/// - 100_000_000 sats (1 BTC)
/// - 10_000_000 sats  (0.1 BTC)
/// - 1_000_000 sats   (0.01 BTC)
/// - 100_000 sats     (0.001 BTC / 1 mBTC)
///
/// Zero-value and OP_RETURN outputs are excluded.
///
/// This heuristic is useful on its own for flagging likely payment outputs,
/// and it also feeds into the change_detection heuristic (where the
/// non-round output is inferred as change).
pub fn detect(tx: &RawTx) -> RoundNumberResult {
    if is_coinbase(tx) {
        return RoundNumberResult {
            detected: false,
            round_output_indices: Vec::new(),
        };
    }

    let round_indices: Vec<usize> = tx
        .outputs
        .iter()
        .enumerate()
        .filter(|(_, o)| {
            let val = o.value as u64;
            // Skip zero-value and OP_RETURN outputs.
            if val == 0 {
                return false;
            }
            if !o.script_pubkey.is_empty() && o.script_pubkey[0] == 0x6a {
                return false;
            }
            is_round(val)
        })
        .map(|(i, _)| i)
        .collect();

    RoundNumberResult {
        detected: !round_indices.is_empty(),
        round_output_indices: round_indices,
    }
}

/// Returns true if the value is divisible by 100_000 sats (1 mBTC) or
/// any coarser denomination.
fn is_round(sats: u64) -> bool {
    sats % 100_000 == 0
}

fn is_coinbase(tx: &RawTx) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].prev_txid == [0u8; 32]
        && tx.inputs[0].vout == 0xFFFF_FFFF
}
