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

## Results (single thread, Intel Xeon @ 2.1 GHz with AVX-512)

Bits per posting are measured on the lists that appear in queries; times are
the best algorithm per codec, in ms per query, with docIDs written out.
Clustered and uniform: 8M documents, 21M postings, 500 queries of 2-4
co-occurring terms. WikiText-103: 859,532 paragraphs, 57M postings, 500
queries.

| codec | clustered bits | clustered AND ms | clustered OR ms | uniform bits | WikiText bits | WikiText AND ms | WikiText OR ms |
|---|---:|---:|---:|---:|---:|---:|---:|
| Uncompressed | 32.00 | 0.061 | 1.86 | 32.00 | 32.00 | 0.022 | 0.561 |
| VByte | 9.61 | 0.133 | 3.20 | 10.91 | 8.91 | 0.051 | 0.974 |
| BP128 | 7.69 | 0.080 | 2.89 | 9.39 | 7.11 | 0.033 | 0.966 |
| BP128 (bitpacking crate) | 7.69 | 0.072 | 2.54 | 9.39 | 7.11 | 0.030 | 0.855 |
| OptPFor | 5.92 | 0.088 | 3.00 | 8.87 | 5.91 | 0.037 | 0.898 |
| Elias-Fano | 8.13 | 0.055 | 2.87 | 8.08 | 5.16 | 0.019 | 0.976 |
| PEF | 4.94 | 0.046 | 2.84 | 8.08 | 4.75 | 0.022 | 0.931 |
| Interpolative | 4.90 | 0.181 | 5.00 | 8.52 | 4.77 | 0.081 | 1.623 |
| Hybrid (bitmap + BP128) | 7.58 | 0.067 | 1.91 | 9.31 | 5.85 | 0.025 | 0.387 |
| Roaring | 9.32 | 0.030 | 0.57 | 13.70 | 8.47 | 0.007 | 0.086 |
| Trie | 7.18 | 0.147 | 2.50 | 13.24 | 6.50 | 0.062 | 0.485 |
| Run-Trie | 6.67 | 0.132 | 2.84 | 13.24 | 6.23 | 0.061 | 0.530 |
| Roaring (roaring crate) | 9.32 | 0.088 | 4.30 | 13.70 | 8.47 | 0.027 | 1.182 |

* Roaring is the fastest for AND and OR on every collection (4.5-10x faster
  OR than the best list codec) at 1.6-1.9x PEF's size.
* PEF is the smallest codec, or within 1% of it, everywhere, and Elias-Fano or
  PEF is the fastest list codec for AND.
* Rarest-first leapfrog (`NextGEQ`) is 4-18x faster than decode-then-intersect.
* SIMD block merge is 4.4x faster than scalar merge on equal-size lists;
  galloping wins beyond a ~1:24 size ratio.
* DocID clustering shrinks PEF, interpolative, Roaring and tries by 32-50%.

Full analysis, formulas and diagrams: [`report/report.html`](report/report.html).

