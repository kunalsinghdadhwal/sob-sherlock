use std::collections::HashSet;

use serde::Serialize;

use crate::analysis::classify::classify_output;
use crate::parser::tx::RawTx;
use crate::parser::undo::UndoCoin;

/// Result of attempting to identify the change output in a transaction.
#[derive(Debug, Clone, Serialize)]
pub struct ChangeResult {
    pub detected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub likely_change_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<&'static str>,
}

impl ChangeResult {
    fn none() -> Self {
        Self {
            detected: false,
            likely_change_index: None,
            method: None,
            confidence: None,
        }
    }

    fn found(index: usize, method: &'static str, confidence: &'static str) -> Self {
        Self {
            detected: true,
            likely_change_index: Some(index),
            method: Some(method),
            confidence: Some(confidence),
        }
    }
}

/// Attempts to identify the likely change output in a Bitcoin transaction.
///
/// Bitcoin transactions do not label which output is the payment and which is
/// the change returning to the sender. This function applies three heuristics
/// in strict priority order:
///
/// 1. **Script type match** (high confidence) -- If all inputs share one
///    script type and exactly one output matches that type while the others
///    differ, that output is very likely change. Wallets almost always create
///    change using the same address format as their inputs.
///
/// 2. **Round number analysis** (medium confidence) -- Payments to others
///    tend to be round amounts (0.1 BTC, 0.01 BTC, etc.) while change is
///    whatever is left over. If exactly one output is a round amount, the
///    other (non-round) output is likely change.
///
/// 3. **Largest output** (low confidence) -- In simple spends the payment is
///    typically smaller than the leftover change. Used only as a last resort.
///
/// Limitations:
/// - CoinJoin/PayJoin transactions break all assumptions.
/// - Batched payments (>2 outputs) reduce reliability.
/// - Skips coinbase transactions (no change concept).
/// - A sophisticated sender can deliberately defeat these heuristics.
///
/// `prevouts` must correspond 1:1 to `tx.inputs` (the spent UTXOs from undo
/// data). For coinbase transactions, pass an empty slice.
pub fn detect(tx: &RawTx, prevouts: &[UndoCoin]) -> ChangeResult {
    // Coinbase: single input spending the null outpoint, no change concept.
    if is_coinbase(tx) {
        return ChangeResult::none();
    }

    if tx.outputs.len() < 2 {
        return ChangeResult::none();
    }

    // --- Method 1: Script type match ---
    if let Some(result) = try_script_type_match(tx, prevouts) {
        return result;
    }

    // --- Method 2: Round number analysis ---
    if let Some(result) = try_round_number(tx) {
        return result;
    }

    // --- Method 3: Largest output (fallback) ---
    try_largest_output(tx)
}

/// Coinbase transactions have a single input with prev_txid = 0x00..00 and
/// vout = 0xFFFFFFFF.
fn is_coinbase(tx: &RawTx) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].prev_txid == [0u8; 32]
        && tx.inputs[0].vout == 0xFFFF_FFFF
}

/// Method 1: If all inputs share one script type and exactly one output
/// matches it (others differ), that output is likely change.
fn try_script_type_match(tx: &RawTx, prevouts: &[UndoCoin]) -> Option<ChangeResult> {
    if prevouts.is_empty() {
        return None;
    }

    // Collect the set of script types across all inputs.
    let input_types: HashSet<&str> = prevouts
        .iter()
        .map(|p| classify_output(&p.script_pubkey))
        .collect();

    // If inputs use mixed types, this method is unreliable.
    if input_types.len() != 1 {
        return None;
    }

    let dominant = *input_types.iter().next().unwrap();

    // Skip if dominant type is unknown or op_return (nonsensical).
    if dominant == "unknown" || dominant == "op_return" {
        return None;
    }

    // Classify each output and find which ones match the input type.
    let matching_indices: Vec<usize> = tx
        .outputs
        .iter()
        .enumerate()
        .filter(|(_, o)| classify_output(&o.script_pubkey) == dominant)
        .map(|(i, _)| i)
        .collect();

    // Exactly one output matches the input type while others differ.
    if matching_indices.len() == 1 {
        return Some(ChangeResult::found(
            matching_indices[0],
            "script_type_match",
            "high",
        ));
    }

    None
}

/// Method 2: If exactly one output is a round amount, the non-round output
/// is likely change (the round one is the payment).
fn try_round_number(tx: &RawTx) -> Option<ChangeResult> {
    let round_flags: Vec<bool> = tx
        .outputs
        .iter()
        .map(|o| is_round(o.value as u64))
        .collect();

    let round_count = round_flags.iter().filter(|&&r| r).count();
    let non_round_count = round_flags.len() - round_count;

    // Exactly one round output and at least one non-round: the non-round
    // outputs include the change. Pick the largest non-round output as
    // the likely change.
    if round_count == 1 && non_round_count >= 1 {
        let change_idx = tx
            .outputs
            .iter()
            .enumerate()
            .filter(|(i, _)| !round_flags[*i])
            .max_by_key(|(_, o)| o.value as u64)
            .map(|(i, _)| i)
            .unwrap();

        return Some(ChangeResult::found(change_idx, "round_number", "medium"));
    }

    // Exactly one non-round output among all-round others: that odd one out
    // is likely the change.
    if non_round_count == 1 && round_count >= 1 {
        let change_idx = round_flags
            .iter()
            .position(|&r| !r)
            .unwrap();

        return Some(ChangeResult::found(change_idx, "round_number", "medium"));
    }

    None
}

/// Method 3: Assume the largest-value output is change.
fn try_largest_output(tx: &RawTx) -> ChangeResult {
    let max_idx = tx
        .outputs
        .iter()
        .enumerate()
        .max_by_key(|(_, o)| o.value as u64)
        .map(|(i, _)| i)
        .unwrap();

    ChangeResult::found(max_idx, "largest_output", "low")
}

/// Returns true if `sats` is a "round" amount -- divisible by one of the
/// common human-friendly BTC denominations.
///
/// Thresholds (from coarsest to finest):
/// - 100_000_000 (1 BTC)
/// - 10_000_000  (0.1 BTC)
/// - 1_000_000   (0.01 BTC)
/// - 100_000     (0.001 BTC / 1 mBTC)
fn is_round(sats: u64) -> bool {
    if sats == 0 {
        return false;
    }
    sats % 100_000 == 0
}
