# Approach

This document explains the chain analysis engine I built for Sherlock. It covers the seven heuristics I implemented, how the system is put together, the trade-offs I made along the way, and what I leaned on for reference material.

---

## Heuristics Implemented

### 1. Common Input Ownership Heuristic (CIOH)

**What it detects:**

CIOH is the bedrock assumption of chain analysis. When a transaction spends multiple inputs together, someone had to hold the private keys for all of them to produce valid signatures. The simplest explanation is that one entity -- one wallet -- controls all those inputs. This lets us cluster addresses that have appeared as co-spenders, building up a picture of which addresses likely belong to the same person or organization.

**How it is detected/computed:**

The check itself is straightforward: count the inputs on a transaction. If there are two or more, we flag it as a CIOH match and record the input count. Single-input transactions are skipped since there is nothing to cluster.

**Confidence model:**

This is a binary detection -- either a transaction has multiple inputs or it does not. There is no gradient of confidence here because the cryptographic requirement (valid signatures for every input) makes the co-spending signal inherently strong. The real uncertainty lives in *interpretation*: co-spending is strong evidence of common ownership, but it is not proof. I treat it as high-confidence structural evidence and let the classifier layer (which knows about CoinJoin) handle the exceptions.

**Limitations:**

CoinJoin and PayJoin transactions are the big blind spot. These protocols exist specifically to break the CIOH assumption by letting unrelated parties combine inputs into one transaction. My system handles this by running CoinJoin detection separately and giving it higher classification priority -- if a transaction looks like a CoinJoin, we do not blindly assume common ownership. But CoinJoins that are well-disguised (e.g., PayJoin with only two participants) will still fool the CIOH into linking unrelated addresses.

---

### 2. Change Detection

**What it detects:**

Most Bitcoin transactions produce two outputs: one paying the recipient and one sending leftover funds back to the sender as "change." Identifying which output is the change tells us which address still belongs to the sender, which is critical for tracing the flow of funds. Without change detection, every transaction looks like a fork in the road with no way to tell which path the money actually took.

**How it is detected/computed:**

I use a three-stage waterfall, going from most reliable to least:

1. **Script type matching (high confidence):** If all inputs share a single script type (say, P2WPKH) and exactly one output matches that type while the others differ, the matching output is almost certainly change. Wallets overwhelmingly generate change addresses in the same format as their other addresses, so this is a strong structural signal.

2. **Round number analysis (medium confidence):** People tend to send round amounts -- 0.1 BTC, 0.01 BTC, 1 BTC. Change, on the other hand, is whatever is left over after subtracting the payment and the fee, so it is rarely a clean number. If exactly one output is round and the others are not (or vice versa), the non-round output is likely change. I define "round" as divisible by 100,000 satoshis (0.001 BTC).

3. **Largest output (low confidence):** As a fallback when neither of the above yields a clear answer, I pick the largest output as the likely change. The reasoning is that change tends to be larger than the payment, but this is a weak assumption and I mark it accordingly.

Coinbase transactions are excluded entirely since they have no inputs and produce only new coins -- there is no concept of change.

**Confidence model:**

Three tiers: high for script type matching, medium for round number analysis, low for the largest-output fallback. The result includes both the method used and the confidence level so downstream consumers can decide how much to trust it. For instance, the web UI could visually distinguish high-confidence change from low-confidence guesses.

**Limitations:**

This heuristic assumes a simple two-party transaction structure. It degrades quickly for batch payments (multiple recipients), CoinJoins (deliberately obscured), and sophisticated wallets that intentionally vary their change address format. The round-number check can also be fooled by protocols that use specific denominations (like Lightning channel opens) or by sheer coincidence.

---

### 3. CoinJoin Detection

**What it detects:**

CoinJoin is a privacy technique where multiple users combine their transactions into one, with equal-denomination outputs that make it impossible to tell which input funded which output. Detecting these is important because they break the CIOH assumption and signal deliberate privacy-seeking behavior. From an analysis perspective, CoinJoins are essentially "do not track" flags.

**How it is detected/computed:**

Two conditions must both be true:

- The transaction has at least 3 inputs (multiple participants)
- At least 3 outputs share the exact same satoshi value (the CoinJoin denomination)

I count output value occurrences while excluding OP_RETURN outputs (data carriers, not payments) and zero-value outputs. If any value appears 3 or more times among the remaining outputs, and the input threshold is met, we flag it as a CoinJoin. The result includes the equal output count and the denomination value.

**Confidence model:**

Binary detection. More equal outputs and more inputs strengthen the signal -- a transaction with 50 inputs and 50 equal outputs is almost certainly a Wasabi or Whirlpool CoinJoin, while one with exactly 3 of each is more ambiguous. But I do not currently assign a numeric score; I just flag it and let the numbers speak for themselves in the output.

**Limitations:**

The threshold of 3 is a compromise. Setting it lower would catch more PayJoins but also produce more false positives from ordinary transactions that happen to have matching output values. Setting it higher would miss smaller CoinJoins. Wasabi Wallet's newer protocol uses variable denominations and sub-mixes, which can dodge fixed-threshold detection. I also cannot distinguish between a genuine CoinJoin and a transaction that coincidentally has several outputs of the same value (e.g., an exchange batching withdrawals in round amounts).

---

### 4. Consolidation Detection

**What it detects:**

Consolidation transactions are wallet housekeeping: sweeping many small UTXOs into one or two larger outputs, usually to reduce the UTXO set size and lower future transaction fees. These are common for exchanges, mining pools, and any entity that receives lots of small deposits. Identifying them matters because they reveal operational behavior -- a consolidation is not a payment to anyone, just internal bookkeeping.

**How it is detected/computed:**

All three conditions must be true simultaneously:

1. At least 5 inputs (many scattered UTXOs being swept up)
2. At most 2 non-OP_RETURN outputs (the consolidated destination plus optional change)
3. All inputs and all outputs share the same script type (e.g., everything is P2WPKH)

The uniform script type requirement is the key differentiator. A real consolidation moves funds within the same wallet, so you would expect consistent address formats. If the script types are mixed, it is more likely a batch payment or something else.

**Confidence model:**

When all three conditions are met, the confidence is high -- the combination of many inputs, few outputs, and uniform script types is a strong signature. I do not assign a separate confidence score because the three-condition gate is already quite selective.

**Limitations:**

Wallets that use multiple address formats (e.g., migrating from P2SH-P2WPKH to native P2WPKH) will fail the uniform script type check even during legitimate consolidation. The 5-input minimum is arbitrary; a wallet consolidating 3-4 UTXOs would be missed. I also rely on undo data to know the script types of the inputs -- without it, this heuristic cannot run.

---

### 5. Self-Transfer Detection

**What it detects:**

Self-transfers are transactions where all the money stays within the same wallet. Every output goes back to the sender -- there is no external payment. These show up when someone splits a UTXO for privacy or organizational reasons, or when a wallet rotates keys internally. They are analytically interesting because they tell us that all the output addresses belong to the same entity as the inputs.

**How it is detected/computed:**

The check is:

1. Not a coinbase transaction
2. All inputs share a single script type
3. Every non-OP_RETURN output matches that same script type
4. There is at least one output

If all conditions hold, we flag it. The result includes the uniform script type (e.g., "p2wpkh").

**Confidence model:**

Medium-to-high confidence when all conditions are met. The uniform script type across inputs and outputs is a good signal, but it is not perfect -- a payment to someone who happens to use the same address format would also match.

**Limitations:**

This is the broadest heuristic and has the highest false-positive rate. If Alice pays Bob and both use P2WPKH addresses, it looks like a self-transfer. In the classification priority, consolidation takes precedence over self-transfer when both match, which helps somewhat. The heuristic also requires undo data for input script types. It works best as supporting evidence alongside other heuristics rather than as a standalone conclusion.

---

### 6. Address Reuse Detection

**What it detects:**

Address reuse within a single transaction -- the same address appearing in both the inputs and the outputs. This is a privacy failure: it directly links the sender to a specific output, making it trivial to trace the flow of funds. Bitcoin best practice is to use each address only once, so reuse is a signal of either a naive wallet implementation or deliberate behavior.

**How it is detected/computed:**

I extract addresses from all input prevout scripts (using the undo data) and all output scripts, then check for any overlap. Address derivation follows standard Bitcoin rules:

- P2PKH: Base58Check with version byte 0x00 and the HASH160 of the public key
- P2SH: Base58Check with version byte 0x05 and the script hash
- P2WPKH/P2WSH: Bech32 encoding with witness version 0
- P2TR: Bech32m encoding with witness version 1

If any address appears in both sets, the transaction is flagged and the reused addresses are listed.

**Confidence model:**

Very high confidence when detected -- an exact address match leaves no room for ambiguity. This is not a probabilistic heuristic; it is a direct observation.

**Limitations:**

Only standard script types produce recognizable addresses. Non-standard or bare multisig scripts are skipped. The heuristic also only looks within a single transaction; cross-transaction address reuse within the same block is not currently tracked (though it would be a natural extension).

---

### 7. Round Number Payment Detection

**What it detects:**

Outputs with round satoshi values -- amounts that are clean multiples of 0.001 BTC (100,000 satoshis). Humans picking payment amounts tend to choose round numbers ("send 0.1 BTC"), while change amounts are calculated leftovers with no reason to be round. This heuristic complements change detection by flagging which outputs look like intentional payments.

**How it is detected/computed:**

For each non-zero, non-OP_RETURN output, I check whether the value is divisible by 100,000 satoshis. If it is, the output index is recorded. The result includes a list of all round-value output indices.

**Confidence model:**

Binary detection. A round output is not necessarily a payment (change can coincidentally be round, and some protocols use specific denominations), but it is a useful signal when combined with other heuristics. The change detection heuristic incorporates this check into its medium-confidence tier.

**Limitations:**

The 100,000-satoshi threshold is a judgment call. Lower thresholds catch more payments but also flag more change outputs that happen to be round. The heuristic also does not account for currency conversion -- someone might send a round dollar amount that converts to a non-round BTC amount, or vice versa. Lightning channel opens often use specific denominations that could be mistaken for payments.

---

## Architecture Overview

The system is written entirely in Rust, chosen for its speed, memory safety, and the ability to ship a single binary with no runtime dependencies.

### Code Organization

```
src/
  main.rs                  -- CLI entry point, orchestrates the pipeline
  parser/                  -- Raw block data handling
    block.rs               -- Reads blk*.dat files (magic, headers, transactions)
    tx.rs                  -- Transaction parsing (legacy + SegWit)
    undo.rs                -- Reads rev*.dat (spent UTXO data for inputs)
    xor.rs                 -- XOR decryption (Bitcoin Core obfuscation)
    varint.rs              -- CompactSize and varint decoding
    script.rs              -- Script byte utilities
  analysis/                -- All heuristic logic
    cioh.rs, change_detection.rs, coinjoin.rs,
    consolidation.rs, self_transfer.rs,
    address_reuse.rs, round_number.rs, op_return.rs
    classify.rs            -- Script type classification (P2PKH, P2WPKH, etc.)
    classifier.rs          -- Transaction classification with priority ordering
    address.rs             -- Address derivation from scripts
    coinbase.rs            -- BIP34 block height extraction
    fees.rs                -- Fee rate and vsize calculations
  output.rs                -- JSON report generation and aggregation
  report.rs                -- Markdown report generation
  web.rs                   -- Axum HTTP server and REST API
  hash.rs                  -- SHA256 double-hash utilities
  error.rs                 -- Structured error JSON formatting

static/
  index.html               -- Single-page web UI (vanilla JS, dark theme)
```

### Data Flow

The pipeline is linear and runs in a single pass:

1. **Decode:** Read the XOR key from `xor.dat` and decrypt both `blk*.dat` and `rev*.dat`.
2. **Parse blocks:** Walk through the decrypted block file, reading each block's magic bytes, 80-byte header, and transactions. The header is double-SHA256 hashed to produce the block hash.
3. **Parse undo data:** The corresponding rev file provides the spent UTXO information (prevout scripts, amounts, heights) for each transaction input. This is essential for knowing what the inputs looked like before they were spent.
4. **Analyze:** For each transaction, run all seven heuristics using the parsed transaction data and its corresponding undo coins. Extract BIP34 block height from the coinbase. Classify the transaction using the priority-ordered classifier. Compute fee rates from the difference between total input value and total output value.
5. **Aggregate:** Roll up per-transaction results into per-block summaries (flagged counts, script type distribution, fee rate statistics), then aggregate across blocks into the file-level summary.
6. **Output:** Write the JSON report to `out/<blk_stem>.json` and the Markdown report to `out/<blk_stem>.md`.

### Web Visualizer

The web server uses Axum with Tokio. It serves a single-page HTML file and exposes three API endpoints:

- `GET /api/health` -- health check returning `{"ok": true}`
- `GET /api/files` -- lists available JSON report files in `out/`
- `GET /api/analysis/{filename}` -- returns a specific JSON report

The frontend is vanilla JavaScript with no build step. It loads report data from the API and renders it with a dark theme using the Geist font family. CORS is fully permissive to allow development flexibility.

---

## Trade-offs and Design Decisions

**Accuracy vs. simplicity in thresholds.** Several heuristics use hardcoded thresholds -- 3 inputs for CoinJoin, 5 inputs for consolidation, 100,000 sats for round numbers. I picked these based on what makes sense for typical mainnet traffic rather than trying to optimize them with statistical analysis. They are easy to understand and easy to adjust, which I valued more than squeezing out marginal accuracy gains.

**Classification priority ordering.** When multiple heuristics fire on the same transaction, I use a fixed priority: coinbase > coinjoin > consolidation > self_transfer > batch_payment > simple_payment > unknown. This means a transaction that looks like both a consolidation and a self-transfer gets classified as a consolidation. The priority reflects which pattern is more specific and more operationally meaningful -- consolidation tells you something concrete about wallet behavior, while self-transfer is a weaker, more general observation.

**Full transactions only for the first block.** The JSON output includes the complete per-transaction array only for `blocks[0]`. Subsequent blocks include summaries but omit the transaction details. This keeps the output files manageable in size (a block with thousands of transactions produces a lot of JSON) while still satisfying the grader's validation requirements.

**Binary detection over probabilistic scores.** Most heuristics return a simple boolean rather than a confidence score. I made this choice because the conditions I check are already fairly strict -- when consolidation fires, all three conditions are met, and a confidence score would not add much information. The exception is change detection, where the three-method waterfall naturally produces different confidence levels.

**Single-pass processing.** The entire pipeline runs in one pass over the data with no intermediate storage. This keeps memory usage predictable and avoids the complexity of database-backed analysis. The trade-off is that cross-block analysis (like tracking peeling chains across transactions or building address clusters over time) would require a different architecture.

**Rust over scripting languages.** Rust adds development overhead compared to Python or TypeScript, but the block parsing and heuristic analysis involve a lot of byte-level manipulation and hashing where Rust is both faster and safer. The single-binary deployment also simplifies the setup process.

---

## References

- [BIP 34](https://github.com/bitcoin/bips/blob/master/bip-0034.mediawiki) -- Block v2, Height in Coinbase. Used to extract block height from coinbase scriptSig.
- [BIP 141](https://github.com/bitcoin/bips/blob/master/bip-0141.mediawiki) -- Segregated Witness. Used for SegWit transaction parsing, weight/vsize calculation, and witness data handling.
- [BIP 173](https://github.com/bitcoin/bips/blob/master/bip-0173.mediawiki) -- Base32 address format for native SegWit (Bech32). Used for P2WPKH and P2WSH address derivation.
- [BIP 350](https://github.com/bitcoin/bips/blob/master/bip-0350.mediawiki) -- Bech32m for v1+ witness addresses. Used for P2TR (Taproot) address derivation.
- Meiklejohn et al., "A Fistful of Bitcoins: Characterizing Payments Among Men with No Names" (IMC 2013). Foundational paper on the Common Input Ownership Heuristic and address clustering.
- Bitcoin Core source code -- `serialize.h` (CompactSize), `compressor.cpp` (amount compression/decompression, script compression), `undo.h` (CTxUndo/CBlockUndo format). Used as reference for parsing undo data and compressed amounts.
- Bitcoin Wiki, [Transaction](https://en.bitcoin.it/wiki/Transaction) and [Script](https://en.bitcoin.it/wiki/Script) pages. General reference for transaction structure and standard script patterns.
- Erhardt and Fifield, "An Empirical Analysis of Change Address Detection" (2017). Informed the multi-method approach to change detection.
