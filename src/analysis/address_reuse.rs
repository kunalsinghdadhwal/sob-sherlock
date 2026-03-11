use std::collections::HashSet;

use serde::Serialize;

use crate::analysis::address::derive_address;
use crate::parser::tx::RawTx;
use crate::parser::undo::UndoCoin;

/// Result of the address reuse heuristic.
#[derive(Debug, Clone, Serialize)]
pub struct AddressReuseResult {
    pub detected: bool,
    /// Addresses that appear in both inputs and outputs of this transaction.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reused_addresses: Vec<String>,
}

/// Detects address reuse within a single transaction.
///
/// Address reuse occurs when the same address appears as both an input
/// (via the prevout script_pubkey) and an output in the same transaction.
/// This is a privacy concern because it explicitly links the sender's
/// identity across inputs and outputs, making chain analysis trivial for
/// those addresses.
///
/// Common causes:
/// - Wallet sending change back to the same address it spent from.
/// - Poorly implemented wallets that do not generate fresh change addresses.
///
/// `prevouts` must correspond 1:1 to `tx.inputs` (spent UTXOs from undo
/// data). For coinbase transactions, pass an empty slice.
pub fn detect(tx: &RawTx, prevouts: &[UndoCoin]) -> AddressReuseResult {
    if is_coinbase(tx) || prevouts.is_empty() {
        return AddressReuseResult {
            detected: false,
            reused_addresses: Vec::new(),
        };
    }

    // Collect all input addresses from prevout scripts.
    let input_addrs: HashSet<String> = prevouts
        .iter()
        .filter_map(|p| derive_address(&p.script_pubkey))
        .collect();

    if input_addrs.is_empty() {
        return AddressReuseResult {
            detected: false,
            reused_addresses: Vec::new(),
        };
    }

    // Check which output addresses also appear in inputs.
    let mut reused: Vec<String> = tx
        .outputs
        .iter()
        .filter_map(|o| derive_address(&o.script_pubkey))
        .filter(|addr| input_addrs.contains(addr))
        .collect();

    // Deduplicate (multiple outputs could reuse the same input address).
    reused.sort();
    reused.dedup();

    AddressReuseResult {
        detected: !reused.is_empty(),
        reused_addresses: reused,
    }
}

fn is_coinbase(tx: &RawTx) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].prev_txid == [0u8; 32]
        && tx.inputs[0].vout == 0xFFFF_FFFF
}
