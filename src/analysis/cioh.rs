use serde::Serialize;

use crate::parser::tx::RawTx;

/// Result of applying the Common Input Ownership Heuristic to a transaction.
#[derive(Debug, Clone, Serialize)]
pub struct CiohResult {
    pub detected: bool,
    pub input_count: usize,
}

/// Applies the Common Input Ownership Heuristic (CIOH).
///
/// CIOH assumes that all inputs spent by a single transaction are controlled
/// by the same entity. The reasoning is straightforward: creating a valid
/// transaction requires the private key for every input being spent, so it
/// is overwhelmingly likely that one wallet holds all of them.
///
/// A transaction with 2 or more inputs is flagged as detected. Single-input
/// transactions (including coinbase) are not flagged.
///
/// Notable false positives: CoinJoin and PayJoin transactions intentionally
/// combine inputs from multiple parties, violating the assumption. These are
/// handled by a separate coinjoin heuristic.
pub fn detect(tx: &RawTx) -> CiohResult {
    let input_count = tx.inputs.len();
    CiohResult {
        detected: input_count >= 2,
        input_count,
    }
}
