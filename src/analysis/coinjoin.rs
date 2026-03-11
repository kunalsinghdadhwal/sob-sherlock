use std::collections::HashMap;

use serde::Serialize;

use crate::parser::tx::RawTx;

/// Result of applying the CoinJoin detection heuristic.
#[derive(Debug, Clone, Serialize)]
pub struct CoinjoinResult {
    pub detected: bool,
    /// Number of outputs sharing the most common equal value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal_output_count: Option<usize>,
    /// The equal output value in satoshis (if detected).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equal_output_value: Option<u64>,
}

/// Minimum number of inputs required to consider CoinJoin.
const MIN_INPUTS: usize = 3;

/// Minimum number of equal-value outputs required.
const MIN_EQUAL_OUTPUTS: usize = 3;

/// Detects potential CoinJoin transactions.
///
/// CoinJoin is a privacy technique where multiple independent users combine
/// their inputs into a single transaction, each producing an output of the
/// same denomination. This breaks the common-input-ownership assumption
/// because the inputs genuinely belong to different entities.
///
/// Detection signals:
/// 1. The transaction has many inputs (>= 3) -- multiple participants.
/// 2. Multiple outputs share the exact same value (>= 3) -- the equal
///    denomination that each participant receives back.
///
/// Both conditions must hold simultaneously. A transaction with many inputs
/// but no equal-value outputs is more likely a consolidation. A transaction
/// with equal outputs but only 1-2 inputs is more likely a batch payment.
///
/// Known limitations:
/// - PayJoin (P2EP) uses only 2 participants and will not be caught here.
/// - Wasabi/Whirlpool coinjoins with unequal outputs or sub-mixes may be
///   missed if the equal-output count falls below the threshold.
/// - A non-CoinJoin transaction could coincidentally have equal outputs.
/// - OP_RETURN outputs are excluded from equal-value counting since they
///   are data carriers (value always 0), not participant payouts.
pub fn detect(tx: &RawTx) -> CoinjoinResult {
    if is_coinbase(tx) || tx.inputs.len() < MIN_INPUTS {
        return CoinjoinResult {
            detected: false,
            equal_output_count: None,
            equal_output_value: None,
        };
    }

    // Count occurrences of each output value, excluding OP_RETURN (0x6a prefix)
    // and zero-value outputs.
    let mut value_counts: HashMap<u64, usize> = HashMap::new();
    for out in &tx.outputs {
        let val = out.value as u64;
        if val == 0 {
            continue;
        }
        // Skip OP_RETURN outputs.
        if !out.script_pubkey.is_empty() && out.script_pubkey[0] == 0x6a {
            continue;
        }
        *value_counts.entry(val).or_insert(0) += 1;
    }

    // Find the value with the highest occurrence count.
    let best = value_counts
        .iter()
        .max_by_key(|(_, count)| **count);

    match best {
        Some((value, count)) if *count >= MIN_EQUAL_OUTPUTS => CoinjoinResult {
            detected: true,
            equal_output_count: Some(*count),
            equal_output_value: Some(*value),
        },
        _ => CoinjoinResult {
            detected: false,
            equal_output_count: None,
            equal_output_value: None,
        },
    }

}

fn is_coinbase(tx: &RawTx) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].prev_txid == [0u8; 32]
        && tx.inputs[0].vout == 0xFFFF_FFFF
}
