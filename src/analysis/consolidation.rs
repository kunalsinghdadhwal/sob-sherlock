use std::collections::HashSet;

use serde::Serialize;

use crate::analysis::classify::classify_output;
use crate::parser::tx::RawTx;
use crate::parser::undo::UndoCoin;

/// Result of applying the consolidation detection heuristic.
#[derive(Debug, Clone, Serialize)]
pub struct ConsolidationResult {
    pub detected: bool,
    pub input_count: usize,
    pub output_count: usize,
    /// Whether all inputs and outputs share a single script type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uniform_script_type: Option<&'static str>,
}

/// Minimum number of inputs to consider consolidation.
/// A consolidation sweeps many small UTXOs into fewer outputs.
const MIN_INPUTS: usize = 5;

/// Maximum number of non-OP_RETURN outputs for consolidation.
/// Typically 1 (pure sweep) or 2 (sweep + change), rarely more.
const MAX_REAL_OUTPUTS: usize = 2;

/// Detects UTXO consolidation transactions.
///
/// Consolidation is when a wallet sweeps many small UTXOs into a single (or
/// very few) outputs, reducing the UTXO set and saving on future fees. The
/// pattern is distinctive:
///
/// 1. Many inputs (>= 5) -- the wallet is combining scattered funds.
/// 2. Very few non-OP_RETURN outputs (1-2) -- just the destination and
///    possibly a small change output.
/// 3. Uniform script type -- all inputs and all outputs use the same address
///    format (e.g., all p2wpkh). This is the strongest signal because the
///    wallet is sending funds back to itself using its own address type.
///
/// All three conditions must hold for a positive detection. If the script
/// types are mixed, it is more likely a batch payment or CoinJoin.
///
/// `prevouts` must correspond 1:1 to `tx.inputs` (spent UTXOs from undo
/// data). For coinbase transactions, pass an empty slice.
pub fn detect(tx: &RawTx, prevouts: &[UndoCoin]) -> ConsolidationResult {
    let input_count = tx.inputs.len();
    let output_count = tx.outputs.len();

    let not_detected = ConsolidationResult {
        detected: false,
        input_count,
        output_count,
        uniform_script_type: None,
    };

    if is_coinbase(tx) || input_count < MIN_INPUTS {
        return not_detected;
    }

    // Count non-OP_RETURN outputs.
    let real_outputs: Vec<&crate::parser::tx::TxOutput> = tx
        .outputs
        .iter()
        .filter(|o| o.script_pubkey.is_empty() || o.script_pubkey[0] != 0x6a)
        .collect();

    if real_outputs.len() > MAX_REAL_OUTPUTS {
        return not_detected;
    }

    // Check if all inputs share a single script type.
    if prevouts.is_empty() {
        return not_detected;
    }

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

    // Check that all real outputs also use the same script type.
    let all_outputs_match = real_outputs
        .iter()
        .all(|o| classify_output(&o.script_pubkey) == dominant);

    if !all_outputs_match {
        return not_detected;
    }

    ConsolidationResult {
        detected: true,
        input_count,
        output_count,
        uniform_script_type: Some(dominant),
    }
}

fn is_coinbase(tx: &RawTx) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].prev_txid == [0u8; 32]
        && tx.inputs[0].vout == 0xFFFF_FFFF
}
