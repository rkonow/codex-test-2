# posting_bench

Compressed posting-list representations, intersection and union algorithms,
and a benchmark testbed that compares them on synthetic and real collections.
Every query result of every algorithm is checked against a reference before
it is timed.

The full write-up, with formulas, diagrams and charts of the measurements, is
in [`report/report.html`](report/report.html); the raw numbers are in
[`results/`](results/).

## What is implemented

### Representations (`src/codecs/`)

| Codec | Family | Idea | Traits |
|---|---|---|---|
| `Uncompressed` | baseline | sorted `u32` array, galloping `NextGEQ` | cursor |
| `VByte` | block | varint d-gaps, 128 per block + skip table | cursor |
| `BP128` | block | one bit width per 128 gaps, SIMD-BP128 vertical layout | cursor |
| `OptPFor` | block | patched frame of reference, optimal width per block | cursor |
| `Interpolative` | block | binary interpolative coding per block | cursor |
| `Elias-Fano` | succinct | low bits verbatim + unary high bits, `select0` skips | cursor |
| `PEF` | succinct | partitioned Elias-Fano, DP-chosen run / bitvector / EF partitions | cursor |
| `Roaring` | bitmap | 2^16 chunks with array / bitmap / run containers, SIMD kernels | cursor, set ops |
| `Hybrid` | bitmap | plain bitmaps for dense lists (u/n <= 8), BP128 otherwise | cursor, set ops |
| `Trie` | succinct | compact binary trie, 2 bits per node, rank navigation | cursor, set ops |
| `Run-Trie` | succinct | trie with full subtrees cut | cursor, set ops |
| `BP128 (bitpacking crate)` | baseline | Tantivy's `bitpacking` crate in the same block framework | cursor |
| `Roaring (roaring crate)` | baseline | the `roaring` crate | cursor, set ops |

All codecs implement `PostingList` (build, size in bits, decode). `CursorList`
adds a cursor with `docid()`, `next()` and `next_geq(x)`; `SetOps` adds
native multi-list intersection and union.

### Algorithms (`src/algorithms/`)

* **Two sorted arrays** (`pairwise`, `simd`): merge, branch-free merge,
  galloping, binary search, Baeza-Yates, SIMD block merge (SSE 4x4, AVX2 8x8,
  AVX-512 16x16), and an adaptive kernel (SIMD below a 64x size ratio,
  galloping above).
* **k sorted arrays** (`multiway`): SvS with any pairwise kernel, Small
  Adaptive (Demaine et al.), Sequential (Barbay & Kenyon).
* **k cursors** (`cursor`): rarest-first leapfrog AND (PISA, Lucene),
  round-robin AND (Leapfrog Triejoin), heap OR, min-scan OR (PISA), bitset
  window OR (Lucene `BooleanScorer`).
* **Union of arrays** (`union`): heap, pairwise fold, bitmap.

## Running

```sh
cargo test --release                       # 56 tests (+3 ignored timing tests): conformance suite for every codec, algorithms
cargo run --release -- codecs              # list codecs
cargo run --release -- run                 # default: clustered synthetic collection, 500 queries
cargo run --release -- run --distribution uniform --codecs "pef,roaring,bp128" --ops and
cargo run --release -- kernels --ratios 1,4,16,64,256,1024
```

`.cargo/config.toml` builds with `-C target-cpu=native`; SIMD paths are
selected at compile time and every one has a portable fallback.

### Data

* `--source synthetic` (default): Zipf list lengths; `--distribution
  clustered` mimics a reordered index (topic regions, runs of consecutive
  docIDs), `uniform` a random docID order.
* `--source ds2i --input coll.docs`: the binary format of ds2i / PISA
  (`[1, num_docs]` followed by `[len, docIDs...]` sequences of `u32`).
* `--source text --input docs.txt`: one document per line.
* Queries: `--queries file` (term ids, or words for text input), or
  generated: `--query-mode cooccur` picks terms that co-occur in a sampled
  document, `random` picks terms uniformly.
* `posting-bench export` writes a synthetic collection and its queries in
  ds2i format, for use with other systems.

Results are printed as Markdown; `--json` / `--csv` save them.

### Reproducing the report

```sh
scripts/prepare_wikitext.sh data           # WikiText-103 via Hugging Face (needs pyarrow)
scripts/run_experiments.sh data/wikitext103.txt
python3 scripts/build_report.py            # report/report.html from results/*.json
```

<!--RESULTS-SUMMARY-->
