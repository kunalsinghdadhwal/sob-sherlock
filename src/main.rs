mod analysis;
mod error;
mod hash;
mod output;
mod parser;
mod report;
mod web;

use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        let err = error::error_json(
            "INVALID_ARGS",
            "Usage: sherlock --block <blk.dat> <rev.dat> <xor.dat> | --web [port]",
        );
        println!("{}", err);
        process::exit(1);
    }

    if args[1] == "--web" {
        let port: u16 = args.get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| {
                env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(3000)
            });
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(web::serve(port));
        return;
    }

    if args[1] == "--block" {
        if args.len() < 5 {
            let err = error::error_json(
                "INVALID_ARGS",
                "Block mode requires: --block <blk.dat> <rev.dat> <xor.dat>",
            );
            println!("{}", err);
            process::exit(1);
        }
        let blk_path = &args[2];
        let rev_path = &args[3];
        let xor_path = &args[4];

        for path in [blk_path, rev_path, xor_path] {
            if !Path::new(path).exists() {
                let err =
                    error::error_json("FILE_NOT_FOUND", &format!("File not found: {}", path));
                println!("{}", err);
                process::exit(1);
            }
        }

        std::fs::create_dir_all("out").ok();

        match process_block(blk_path, rev_path, xor_path) {
            Ok(()) => process::exit(0),
            Err(e) => {
                let err = error::error_json("BLOCK_PARSE_ERROR", &e.to_string());
                println!("{}", err);
                process::exit(1);
            }
        }
    } else {
        let err = error::error_json("INVALID_ARGS", "Unknown command. Use --block");
        println!("{}", err);
        process::exit(1);
    }
}

fn process_block(
    blk_path: &str,
    rev_path: &str,
    xor_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let xor_key = parser::xor::read_xor_key(xor_path)?;
    let blk_raw = std::fs::read(blk_path)?;
    let rev_raw = std::fs::read(rev_path)?;

    let blk_data = parser::xor::xor_decode(&blk_raw, &xor_key);
    let rev_data = parser::xor::xor_decode(&rev_raw, &xor_key);

    let blocks = parser::block::parse_blk_file(&blk_data)?;
    let undos = parser::undo::parse_rev_file(&rev_data, blocks.len())?;

    let blk_stem = Path::new(blk_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("block");
    let blk_filename = Path::new(blk_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("block.dat");

    eprintln!(
        "Parsed {} blocks from {}, {} undo records",
        blocks.len(),
        blk_filename,
        undos.len()
    );

    let mut block_analyses = Vec::with_capacity(blocks.len());

    for (bi, block) in blocks.iter().enumerate() {
        let undo = &undos[bi];

        let height = if !block.transactions.is_empty() {
            analysis::coinbase::decode_bip34_height(
                &block.transactions[0].inputs[0].script_sig,
            )
        } else {
            0
        };

        let block_hash = hash::to_display_hex(&block.header.block_hash);

        eprintln!(
            "  Analyzing block {}: hash={}, height={}, txs={}",
            bi, block_hash, height, block.transactions.len()
        );

        let mut ba = output::BlockAnalysis::new(block_hash, height);

        for (ti, tx) in block.transactions.iter().enumerate() {
            let is_coinbase = ti == 0;
            let prevouts = if is_coinbase {
                &[][..]
            } else if ti - 1 < undo.tx_undos.len() {
                &undo.tx_undos[ti - 1].prevouts
            } else {
                &[][..]
            };

            // Run all heuristics.
            let h = analysis::classifier::HeuristicResults {
                cioh: analysis::cioh::detect(tx),
                change_detection: analysis::change_detection::detect(tx, prevouts),
                coinjoin: analysis::coinjoin::detect(tx),
                consolidation: analysis::consolidation::detect(tx, prevouts),
                address_reuse: analysis::address_reuse::detect(tx, prevouts),
                self_transfer: analysis::self_transfer::detect(tx, prevouts),
                round_number_payment: analysis::round_number::detect(tx),
            };

            let classification = analysis::classifier::classify(tx, &h);

            // Compute fee rate for non-coinbase transactions.
            let fee_rate = if !is_coinbase && !prevouts.is_empty() {
                let fee_info = analysis::fees::compute_fees(tx, prevouts);
                Some(fee_info.fee_rate_sat_vb)
            } else {
                None
            };

            // Collect output script types.
            let output_types: Vec<&str> = tx
                .outputs
                .iter()
                .map(|o| analysis::classify::classify_output(&o.script_pubkey))
                .collect();

            ba.add_tx(tx.txid.clone(), h, classification, fee_rate, &output_types);
        }

        block_analyses.push(ba);
    }

    // Generate markdown report (before build_report consumes block_analyses).
    let markdown = report::generate_markdown(blk_filename, &block_analyses);
    let md_path = format!("out/{}.md", blk_stem);
    std::fs::write(&md_path, &markdown)?;
    eprintln!("Wrote {}", md_path);

    // Build and write JSON report.
    let report = output::build_report(blk_filename, block_analyses);
    let json = serde_json::to_string_pretty(&report)?;
    let json_path = format!("out/{}.json", blk_stem);
    std::fs::write(&json_path, &json)?;
    eprintln!("Wrote {}", json_path);

    Ok(())
}
