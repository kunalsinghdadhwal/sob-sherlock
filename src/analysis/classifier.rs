use serde::Serialize;

use crate::analysis::address_reuse::AddressReuseResult;
use crate::analysis::change_detection::ChangeResult;
use crate::analysis::cioh::CiohResult;
use crate::analysis::coinjoin::CoinjoinResult;
use crate::analysis::consolidation::ConsolidationResult;
use crate::analysis::round_number::RoundNumberResult;
use crate::analysis::self_transfer::SelfTransferResult;
use crate::parser::tx::RawTx;

/// All heuristic results for a single transaction, plus its classification.
#[derive(Debug, Clone, Serialize)]
pub struct TxAnalysis {
    pub txid: String,
    pub heuristics: HeuristicResults,
    pub classification: &'static str,
}

/// The complete set of heuristic results for one transaction.
#[derive(Debug, Clone, Serialize)]
pub struct HeuristicResults {
    pub cioh: CiohResult,
    pub change_detection: ChangeResult,
    pub coinjoin: CoinjoinResult,
    pub consolidation: ConsolidationResult,
    pub address_reuse: AddressReuseResult,
    pub self_transfer: SelfTransferResult,
    pub round_number_payment: RoundNumberResult,
}

impl HeuristicResults {
    /// Returns true if any heuristic fired (detected == true).
    pub fn any_detected(&self) -> bool {
        self.cioh.detected
            || self.change_detection.detected
            || self.coinjoin.detected
            || self.consolidation.detected
            || self.address_reuse.detected
            || self.self_transfer.detected
            || self.round_number_payment.detected
    }
}

/// Classifies a transaction based on its heuristic results.
///
/// Priority order (first match wins):
/// 1. **coinbase** -- the block reward transaction (no real inputs).
/// 2. **coinjoin** -- multiple participants mixing coins; takes priority
///    because it invalidates most other heuristics (CIOH, change detection).
/// 3. **consolidation** -- many inputs swept into 1-2 outputs, uniform
///    script type. This is a specific subset of self-transfer.
/// 4. **self_transfer** -- all outputs match input script type, no payment
///    component visible. Checked after consolidation so consolidation gets
///    its own label.
/// 5. **batch_payment** -- more than 2 non-OP_RETURN outputs that are NOT
///    coinjoin/consolidation/self-transfer. Likely a service paying multiple
///    recipients at once.
/// 6. **simple_payment** -- the common case: 1-2 outputs, change detected
///    or single output.
/// 7. **unknown** -- nothing matched clearly.
pub fn classify(tx: &RawTx, h: &HeuristicResults) -> &'static str {
    // Coinbase: single null-outpoint input.
    if is_coinbase(tx) {
        return "coinbase";
    }

    // CoinJoin takes top priority -- it breaks other heuristic assumptions.
    if h.coinjoin.detected {
        return "coinjoin";
    }

    // Consolidation: many inputs, few outputs, uniform type.
    if h.consolidation.detected {
        return "consolidation";
    }

    // Self-transfer: all outputs match input type, no payment visible.
    if h.self_transfer.detected {
        // Distinguish from batch: if there are many outputs it could be
        // a self-transfer splitting UTXOs, but if outputs <= 2 it is
        // a straightforward self-transfer.
        let real_outputs = count_real_outputs(tx);
        if real_outputs <= 2 {
            return "self_transfer";
        }
        // Many outputs all same type could still be self-transfer
        // (UTXO splitting), but we label it as such.
        return "self_transfer";
    }

    // Batch payment: 3+ real outputs that are not coinjoin/consolidation/self-transfer.
    let real_outputs = count_real_outputs(tx);
    if real_outputs > 2 {
        return "batch_payment";
    }

    // Simple payment: typical 1-2 output transaction.
    if real_outputs >= 1 {
        return "simple_payment";
    }

    "unknown"
}

/// Count non-OP_RETURN outputs.
fn count_real_outputs(tx: &RawTx) -> usize {
    tx.outputs
        .iter()
        .filter(|o| o.script_pubkey.is_empty() || o.script_pubkey[0] != 0x6a)
        .count()
}

fn is_coinbase(tx: &RawTx) -> bool {
    tx.inputs.len() == 1
        && tx.inputs[0].prev_txid == [0u8; 32]
        && tx.inputs[0].vout == 0xFFFF_FFFF
}
