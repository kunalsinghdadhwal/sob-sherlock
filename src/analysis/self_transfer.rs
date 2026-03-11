use std::collections::HashSet;

use serde::Serialize;

use crate::analysis::classify::classify_output;
use crate::parser::tx::RawTx;
use crate::parser::undo::UndoCoin;

/// Result of the self-transfer detection heuristic.
#[derive(Debug, Clone, Serialize)]
pub struct SelfTransferResult {
    pub detected: bool,
    /// The uniform script type shared by all inputs and outputs (if detected).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform_script_type: Option<&'static str>,
}

/// Detects likely self-transfers (moving funds within the same wallet).
///
/// A self-transfer is when all non-OP_RETURN outputs use the same script
/// type as all inputs, and there is no obvious "payment" component going
/// to a different address format. This suggests the user is reorganizing
/// funds within their own wallet rather than paying someone else.
///
/// Conditions (all must hold):
/// 1. Not a coinbase transaction.
/// 2. At least 1 output.
/// 3. All inputs share a single script type.
/// 4. Every non-OP_RETURN output matches that same script type.
/// 5. Not already flagged as consolidation (consolidation requires >= 5
///    inputs; self-transfer covers the lower-input-count case with 2+
///    outputs where the pattern still holds).
///
/// The key difference from consolidation: self-transfer can have many
/// outputs (e.g., splitting a UTXO into multiple same-type outputs for
/// future use) and does not require a high input count.
///
/// `prevouts` must correspond 1:1 to `tx.inputs`.
pub fn detect(tx: &RawTx, prevouts: &[UndoCoin]) -> SelfTransferResult {
    let not_detected = SelfTransferResult {
        detected: false,
        uniform_script_type: None,
    };

    if is_coinbase(tx) || prevouts.is_empty() || tx.outputs.is_empty() {
        return not_detected;
    }

    // All inputs must share a single known script type.
    let input_types: HashSet<&str> = prevouts
        .iter()
        .map(|p| classify_output(&p.script_pubkey))
        .collect();

    if input_types.len() != 1 {
        return not_detected;
    }

    let dominant = *input_types.iter().next().unwrap();

    if dominant == "unknown" || dominant == "op_return" {
        return not_detected;
    }

    // Every non-OP_RETURN output must match the input type.
    let real_outputs: Vec<&crate::parser::tx::TxOutput> = tx
        .outputs
        .iter()
        .filter(|o| {
            o.script_pubkey.is_empty() || o.script_pubkey[0] != 0x6a
        })
        .collect();

    if real_outputs.is_empty() {
        return not_detected;
    }

    let all_match = real_outputs
        .iter()
        .all(|o| classify_output(&o.script_pubkey) == dominant);

    if !all_match {
        return not_detected;
    }

    SelfTransferResult {
        detected: true,
        uniform_script_type: Some(dominant),
    }
}

fn is_coinbase(tx: &RawTx) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].prev_txid == [0u8; 32]
        && tx.inputs[0].vout == 0xFFFF_FFFF
}
