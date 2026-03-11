mod analysis;
mod error;
mod hash;
mod parser;

use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        let err = error::error_json(
            "INVALID_ARGS",
            "Usage: sherlock --block <blk.dat> <rev.dat> <xor.dat>",
        );
        println!("{}", err);
        process::exit(1);
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

fn process_block(blk_path: &str, rev_path: &str, xor_path: &str) -> Result<(), Box<dyn std::error::Error>> {
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

    // Verify parsing worked
    eprintln!(
        "Parsed {} blocks from {}, {} undo records",
        blocks.len(),
        blk_filename,
        undos.len()
    );
    for (i, block) in blocks.iter().enumerate() {
        let height = if !block.transactions.is_empty() {
            analysis::coinbase::decode_bip34_height(&block.transactions[0].inputs[0].script_sig)
        } else {
            0
        };
        eprintln!(
            "  Block {}: hash={}, height={}, txs={}",
            i,
            hash::to_display_hex(&block.header.block_hash),
            height,
            block.transactions.len()
        );
    }

    // Analyze first block with classifier to verify.
    if let (Some(block), Some(undo)) = (blocks.first(), undos.first()) {
        let mut class_counts = std::collections::HashMap::<&str, usize>::new();
        let mut flagged = 0usize;

        for (ti, tx) in block.transactions.iter().enumerate() {
            let prevouts = if ti == 0 {
                &[][..]
            } else if ti - 1 < undo.tx_undos.len() {
                &undo.tx_undos[ti - 1].prevouts
            } else {
                &[][..]
            };

            let h = analysis::classifier::HeuristicResults {
                cioh: analysis::cioh::detect(tx),
                change_detection: analysis::change_detection::detect(tx, prevouts),
                coinjoin: analysis::coinjoin::detect(tx),
                consolidation: analysis::consolidation::detect(tx, prevouts),
                address_reuse: analysis::address_reuse::detect(tx, prevouts),
                self_transfer: analysis::self_transfer::detect(tx, prevouts),
                round_number_payment: analysis::round_number::detect(tx),
            };

            if h.any_detected() {
                flagged += 1;
            }

            let class = analysis::classifier::classify(tx, &h);
            *class_counts.entry(class).or_insert(0) += 1;
        }

        eprintln!(
            "  Block 0 ({} txs): flagged={}",
            block.transactions.len(),
            flagged,
        );
        let mut sorted: Vec<_> = class_counts.iter().collect();
        sorted.sort_by_key(|(_, v)| std::cmp::Reverse(**v));
        for (class, count) in &sorted {
            eprintln!("    {}: {}", class, count);
        }
    }

    // TODO: build JSON output, write to out/<blk_stem>.json
    // TODO: generate markdown report, write to out/<blk_stem>.md
    let _ = blk_stem;

    Ok(())
}
