use std::collections::HashMap;

use serde::Serialize;

use crate::analysis::classifier::{HeuristicResults, TxAnalysis};

/// Top-level JSON output for a block file.
#[derive(Debug, Serialize)]
pub struct FileReport {
    pub ok: bool,
    pub mode: &'static str,
    pub file: String,
    pub block_count: usize,
    pub analysis_summary: AnalysisSummary,
    pub blocks: Vec<BlockEntry>,
}

/// Summary statistics (used at both file and block level).
#[derive(Debug, Clone, Serialize)]
pub struct AnalysisSummary {
    pub total_transactions_analyzed: usize,
    pub heuristics_applied: Vec<&'static str>,
    pub flagged_transactions: usize,
    pub script_type_distribution: HashMap<String, usize>,
    pub fee_rate_stats: FeeRateStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeeRateStats {
    pub min_sat_vb: f64,
    pub max_sat_vb: f64,
    pub median_sat_vb: f64,
    pub mean_sat_vb: f64,
}

/// One block's entry in the blocks array.
#[derive(Debug, Serialize)]
pub struct BlockEntry {
    pub block_hash: String,
    pub block_height: u64,
    pub tx_count: usize,
    pub analysis_summary: AnalysisSummary,
    /// Required for blocks[0], optional for the rest.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<TxAnalysis>>,
}

const HEURISTICS_APPLIED: [&str; 7] = [
    "cioh",
    "change_detection",
    "coinjoin",
    "consolidation",
    "address_reuse",
    "self_transfer",
    "round_number_payment",
];

/// Holds per-block analysis results before building the final report.
pub struct BlockAnalysis {
    pub block_hash: String,
    pub block_height: u64,
    pub tx_count: usize,
    pub tx_analyses: Vec<TxAnalysis>,
    pub fee_rates: Vec<f64>,
    pub script_type_counts: HashMap<String, usize>,
    pub flagged: usize,
}

impl BlockAnalysis {
    pub fn new(block_hash: String, block_height: u64) -> Self {
        Self {
            block_hash,
            block_height,
            tx_count: 0,
            tx_analyses: Vec::new(),
            fee_rates: Vec::new(),
            script_type_counts: HashMap::new(),
            flagged: 0,
        }
    }

    pub fn add_tx(
        &mut self,
        txid: String,
        heuristics: HeuristicResults,
        classification: &'static str,
        fee_rate: Option<f64>,
        output_script_types: &[&str],
    ) {
        if heuristics.any_detected() {
            self.flagged += 1;
        }

        if let Some(rate) = fee_rate {
            self.fee_rates.push(rate);
        }

        for st in output_script_types {
            *self.script_type_counts.entry(st.to_string()).or_insert(0) += 1;
        }

        self.tx_analyses.push(TxAnalysis {
            txid,
            heuristics,
            classification,
        });

        self.tx_count += 1;
    }

    fn build_summary(&self) -> AnalysisSummary {
        AnalysisSummary {
            total_transactions_analyzed: self.tx_count,
            heuristics_applied: HEURISTICS_APPLIED.to_vec(),
            flagged_transactions: self.flagged,
            script_type_distribution: self.script_type_counts.clone(),
            fee_rate_stats: compute_fee_stats(&self.fee_rates),
        }
    }
}

/// Build the full FileReport, consuming block_analyses.
pub fn build_report(
    filename: &str,
    block_analyses: Vec<BlockAnalysis>,
) -> FileReport {
    let mut total_txs = 0usize;
    let mut total_flagged = 0usize;
    let mut all_fee_rates: Vec<f64> = Vec::new();
    let mut all_script_types: HashMap<String, usize> = HashMap::new();

    for ba in &block_analyses {
        total_txs += ba.tx_count;
        total_flagged += ba.flagged;
        all_fee_rates.extend_from_slice(&ba.fee_rates);
        for (k, v) in &ba.script_type_counts {
            *all_script_types.entry(k.clone()).or_insert(0) += v;
        }
    }

    let file_summary = AnalysisSummary {
        total_transactions_analyzed: total_txs,
        heuristics_applied: HEURISTICS_APPLIED.to_vec(),
        flagged_transactions: total_flagged,
        script_type_distribution: all_script_types,
        fee_rate_stats: compute_fee_stats(&all_fee_rates),
    };

    let mut blocks = Vec::with_capacity(block_analyses.len());
    for (i, ba) in block_analyses.into_iter().enumerate() {
        let summary = ba.build_summary();
        let transactions = if i == 0 {
            Some(ba.tx_analyses)
        } else {
            None
        };

        blocks.push(BlockEntry {
            block_hash: ba.block_hash,
            block_height: ba.block_height,
            tx_count: ba.tx_count,
            analysis_summary: summary,
            transactions,
        });
    }

    FileReport {
        ok: true,
        mode: "chain_analysis",
        file: filename.to_string(),
        block_count: blocks.len(),
        analysis_summary: file_summary,
        blocks,
    }
}

fn compute_fee_stats(rates: &[f64]) -> FeeRateStats {
    if rates.is_empty() {
        return FeeRateStats {
            min_sat_vb: 0.0,
            max_sat_vb: 0.0,
            median_sat_vb: 0.0,
            mean_sat_vb: 0.0,
        };
    }

    let mut sorted = rates.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let sum: f64 = sorted.iter().sum();
    let mean = (sum / sorted.len() as f64 * 100.0).round() / 100.0;

    let median = if sorted.len() % 2 == 0 {
        let mid = sorted.len() / 2;
        ((sorted[mid - 1] + sorted[mid]) / 2.0 * 100.0).round() / 100.0
    } else {
        sorted[sorted.len() / 2]
    };

    FeeRateStats {
        min_sat_vb: min,
        max_sat_vb: max,
        median_sat_vb: median,
        mean_sat_vb: mean,
    }
}
