# Add benchmark and unwhack bulk-copy + fast-match u64 extension

This PR adds a Criterion benchmark and two performance improvements:

- benches/whack_unwhack.rs: Criterion benchmarks for whackblock and unwhack on 64KiB inputs (zeros and pseudo-random).
- src/unwhack.rs: use ptr::copy (memmove semantics) to replay match sequences with a bulk overlapped copy instead of pushing bytes one-by-one.
- src/whack.rs: fast match-extension using unaligned u64 word comparisons and endian-aware bit-scan to find the first differing byte, significantly reducing per-byte overhead when matches are long.

Notes

- The fast match extension uses unsafe unaligned reads (ptr::read_unaligned). This is unconditional and relies on the behavior being acceptable on common targets (x86/x86_64). If needed we can gate this behind a feature or a #[cfg(...)] target check.
- Tests (existing proptest + ground-truth vectors) should validate correctness. Please run `cargo test --release` locally.

Benchmark results

(Please run `cargo bench` locally to generate results for your machine. I did not include machine-specific numbers in the PR.)
