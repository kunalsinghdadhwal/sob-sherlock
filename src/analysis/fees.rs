use crate::parser::tx::RawTx;
use crate::parser::undo::UndoCoin;

#[derive(Debug, Clone)]
pub struct FeeInfo {
    pub total_input_sats: u64,
    pub total_output_sats: u64,
    pub fee_sats: u64,
    pub size_bytes: usize,
    pub weight: usize,
    pub vbytes: usize,
    pub fee_rate_sat_vb: f64,
}

pub fn compute_fees(tx: &RawTx, prevouts: &[UndoCoin]) -> FeeInfo {
    let total_input_sats: u64 = prevouts.iter().map(|p| p.amount).sum();
    let total_output_sats: u64 = tx.outputs.iter().map(|o| o.value as u64).sum();
    let fee_sats = total_input_sats.saturating_sub(total_output_sats);

    let size_bytes = tx.size;
    let weight = tx.weight;
    let vbytes = (weight + 3) / 4;

    let fee_rate_sat_vb = if vbytes > 0 {
        ((fee_sats as f64 / vbytes as f64) * 100.0).round() / 100.0
    } else {
        0.0
    };

    FeeInfo {
        total_input_sats,
        total_output_sats,
        fee_sats,
        size_bytes,
        weight,
        vbytes,
        fee_rate_sat_vb,
    }
}
