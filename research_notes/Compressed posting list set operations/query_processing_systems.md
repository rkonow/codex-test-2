# Union (OR) and Conjunctive (AND) Query Processing over Compressed Posting Lists: Ranked Retrieval and Production Engines (state as of Oct 2026)

Scope note: These notes cover query-processing strategies and system implementations. Codec design, the theory of intersection algorithms, and SIMD/GPU kernels are covered by other researchers and appear here only as context. All dates are publication or release dates. Latencies are single-threaded and in-memory unless stated otherwise.

## Q1. Traversal strategies: DAAT vs. TAAT vs. SAAT/impact-ordered (JASS, Anytime ranking), and the role of skips/NextGEQ on compressed lists

### Takeaway
Document-at-a-time (DAAT) over docID-ordered, block-compressed lists with skip data (NextGEQ) is still the default in research systems (PISA) and in every production engine reviewed. Impact-ordered score-at-a-time (SAAT; JASS) wins when latency must be strictly bounded (anytime and tail-latency SLAs) and when learned-sparse "wacky weights" defeat DAAT skipping. Since 2023 the strongest systems are hybrids: they run DAAT across blocks or windows and term-at-a-time (TAAT) or bitset accumulation inside fixed docID windows. Examples are Lucene 10.3+, Tantivy, iresearch, BMP and Seismic.

### Cited Findings
- **Index organizations (Mackenzie, Petri, Moffat, "Anytime Ranking on Document-Ordered Indexes", ACM TOIS 40(1), arXiv Apr 2021):**
  - Document-ordered indexes are the most common because they support many query types, support index updates, and allow efficient dynamic pruning. Their weakness is that high-scoring documents are spread across the docID space, so early termination risks effectiveness. — [Mackenzie et al. 2021](https://arxiv.org/pdf/2104.08976)
  - NextGEQ uses skip information stored at block boundaries. That information allows "bidirectional seeking to any arbitrary block efficiently". — [Mackenzie et al. 2021](https://arxiv.org/pdf/2104.08976)
  - In impact-ordered indexes, each postings list is a sequence of segments. Each segment holds one integer impact (a b-bit quantization of the term's score contribution) followed by sorted docIDs; b = 8–10 bits is sufficient for large collections. — [Mackenzie et al. 2021](https://arxiv.org/pdf/2104.08976)
  - SaaT processes segments from all terms in strictly non-increasing impact order and adds the impact to per-document accumulators. Early termination usually caps the number of postings processed. Quantized SaaT is non-safe, with fidelity set by the quantization level. — [Mackenzie et al. 2021](https://arxiv.org/pdf/2104.08976)
- **Anytime ranking on document-ordered indexes:**
  - Documents are reordered into topically coherent ranges, using a cluster-skipping inverted index plus secondary BP (recursive graph bisection) reordering. Ranges are processed in decreasing order of a cheap query-dependent upper-bound score ("BoundSum"), and a heuristic stops processing to meet an SLA. — [Mackenzie et al. 2021](https://arxiv.org/pdf/2104.08976)
  - Over most of the likely response-time range, this approach beats SaaT on an impact-ordered index, and it still supports Boolean conjunction. — [Mackenzie et al. 2021](https://arxiv.org/pdf/2104.08976)
- **Reordering also speeds up SaaT:** on ClueWeb09B with k=10, BP-reordering JASS's impact-ordered index gave 1.62–1.76× speedups at P50/P95/P99. JASS-exhaustive P50/P95/P99 went from 106.7/345.3/489.9 ms (random IDs) to 60.5/201.7/296.1 ms (reordered). The reason is that high-impact postings for a query's terms fall into narrower docID ranges. — [Mackenzie et al. 2021](https://arxiv.org/pdf/2104.08976)
- **Mallia, Siedlaczek, Suel (ECIR 2019):** DAAT query cost "increases significantly with k". The authors say this justifies TAAT and SAAT approaches for candidate generation. — [Mallia et al. ECIR 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **"Wacky weights" (Mackenzie, Trotman, Lin; arXiv Oct 2021, SIGIR 2022 short):**
  - Transformer-based learned sparse models produce weights that "greatly reduce the opportunities for skipping and early exiting" in DAAT. This makes SAAT more competitive.
  - Accepting up to 3% effectiveness loss, SAAT gives substantial mean-latency gains and dramatically lower tail latency.
  - Quantitative results are under Q3. — [arXiv 2110.11540](https://arxiv.org/html/2110.11540v2)
- **Hybrid "DAAT across windows, TAAT within windows" in Lucene:**
  - Lucene 10.3.0 (2025-09-13) "optimize[d] top-n bulk scorers by evaluating scoring windows in a term-at-a-time fashion instead of doc-at-a-time" (GITHUB#14701). It also added `Scorer#nextDocsAndScores` batch iteration (GITHUB#14709). — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
  - Lucene's exhaustive disjunction scorer ("BS1") splits the docID space into windows and processes one clause at a time per window. It sets bits in a bitset and stores scores in an array, then iterates the set bits. Elastic's Sept 2023 description gives a 2,048-doc window. — [Elastic blog, Sept 2023](https://www.elastic.co/blog/speedups-top-k-queries-high-frequency-terms)
  - Current Lucene source uses `SHIFT = 12` (4,096-doc windows) in `BooleanScorer`. `MaxScoreBulkScorer.INNER_WINDOW_SIZE = 1 << 12`, and `DenseConjunctionBulkScorer.WINDOW_SIZE = 4096`. — [BooleanScorer.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/BooleanScorer.java); [MaxScoreBulkScorer.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/MaxScoreBulkScorer.java); [DenseConjunctionBulkScorer.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/DenseConjunctionBulkScorer.java)
  - This bitset-window approach goes back to Doug Cutting's 1997 "Space Optimizations for Total Ranking", where it is called "Block Merge". The heap-based approach there is called "Parallel Merge". — [Elastic blog, Sept 2023](https://www.elastic.co/blog/speedups-top-k-queries-high-frequency-terms)
- **iresearch (C++ engine behind SereneDB):** describes itself in the search-benchmark-game as "Doc-at-a-time for blocks, Term-at-a-time in blocks, in other words Block-at-a-time". — [search-benchmark-game results.json](https://github.com/quickwit-oss/search-benchmark-game)
- **Compiled vs. vectorized evaluation (Adrien Grand, Aug 28, 2025):**
  - Engines with pluggable clauses (Lucene, Tantivy, Vespa) run DAAT over an iterator abstraction, so `advance(target)` and scoring become virtual calls.
  - PISA avoids that overhead because it supports only term clauses, which lets the compiler inline hot code.
  - Lucene's answer is vectorization: batches aligned with postings blocks, `nextDocsAndScores`, and a branchless, SIMD `filterByScore`. — [jpountz blog, Aug 2025](https://jpountz.github.io/2025/08/28/compiled-vs-vectorized-search-engine-edition.html)

### Inferences
- The old DAAT-vs-TAAT dichotomy has blurred. Production engines now use DAAT to choose which window or block to visit, using skip and block-max metadata. Inside a 4,096-doc window they run TAAT-style accumulation into a bitset plus a score array. BMP, Seismic and superblock methods (Q3) are the research counterpart: block-at-a-time selection followed by dense, forward-index-style scoring inside each block.
- SAAT/impact-ordered evaluation remains the natural choice when a hard latency budget is the main constraint. Anytime DAAT over topically clustered, BP-reordered indexes competes with it while keeping a single docID-ordered index.

### Gaps
- No 2024–2026 head-to-head of JASS/SAAT against the newest block-based learned-sparse methods (BMP, SP/LSP, Seismic) under strict tail-latency SLAs was found.
- Anytime-ranking latency curves were not extracted beyond the reordering table.

## Q2. Dynamic pruning for disjunctions and conjunctions (MaxScore, WAND, BMW, BMM, VBMW, BMA/Ranked AND) and experimental comparisons

### Takeaway
On classic BM25 workloads (Gov2/ClueWeb09, TREC queries), block-max methods win for small k. VBMW was the fastest DAAT method in the ECIR 2019 study, while MaxScore wins for k > 100 and for long queries. Lucene moved from block-max WAND (2019) to block-max MaxScore (2022–2023) because MaxScore has lower per-document overhead and degrades gracefully for many-term, high-k and learned-sparse queries. Document reordering (URL or BP) is worth up to about 3× on its own.

### Cited Findings
- **Algorithm definitions:**
  - MaxScore (Turtle & Flood) sorts terms by their max impact and splits them into "essential" and "non-essential" lists. Documents that contain only non-essential terms cannot enter the top k. — [Mallia et al. ECIR 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
  - WAND (Broder et al. 2003) sorts the term cursors by current docID at each step to find a pivot. — [Mallia et al. ECIR 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
  - Block-Max WAND (Ding & Suel 2011) adds per-block maximum scores because list-wide maxima overestimate scores. VBMW generalizes BMW to variable-length blocks. Block-Max MaxScore (BMM) applies per-block maxima to MaxScore. — [Mallia et al. ECIR 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **VBMW (Mallia, Ottaviano, Porciani, Tonellotto, Venturini; SIGIR 2017, pp. 625–634):**
  - Block partitioning is posed as an optimization problem, solved with an approximation algorithm with provable guarantees.
  - It "significantly outperforms the state of the art roughly by a factor 2x".
  - Compressing the extra block metadata cuts its space by about 50% for at most about a 10% slowdown. — [CNR record / abstract](https://iris.cnr.it/handle/20.500.14243/344721)
- **ECIR 2019 setup (Mallia, Siedlaczek, Suel, "An Experimental Study of Index Compression and DAAT Query Processing Methods"):**
  - Collections: Gov2 (24.6M docs, 5.74B postings) and ClueWeb09 (50.1M docs, 15.86B postings).
  - Queries: TREC05/06 efficiency queries, BM25 scoring, PISA codebase.
  - BMW and BMM use 128-posting blocks; VBMW uses an average block length of 40. — [Mallia et al. ECIR 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **ECIR 2019 findings:** — [Mallia et al. ECIR 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
  - VBMW is the fastest algorithm overall. VBMW beats BMW, and BMW beats WAND.
  - MaxScore is fastest on randomly ordered indexes but gains less from reordering. BMM gives "no improvement over MaxScore".
  - Random docID ordering can be "almost 3 times slower" than URL ordering. BP adds further gains over URL, which are consistent on ClueWeb09.
  - MaxScore does better on long queries: about 10 ms faster on ClueWeb09 long queries. The authors suggest hybrid algorithm selection by query length.
  - VBMW and MaxScore are about equal at k = 100. For larger k, MaxScore is faster.
  - SIMD-BP128 matches Varint-G8IU/VarintGB speed with better compression. PEF nearly matches the fastest codecs with VBMW/BMW because those algorithms are skip/lookup-heavy.
- **Lucene history and rationale (Adrien Grand, Elastic, Dec 6, 2023):**
  - WAND usually evaluates fewer hits but has higher per-hit overhead. MAXSCORE generally does better for high k or many terms.
  - Lucene first implemented block-max WAND, then "started using block-max MAXSCORE for top-level disjunctions in July 2022" because of its lower overhead. — [Elastic Search Labs](https://www.elastic.co/search-labs/blog/more-skipping-with-bm-maxscore)
  - The changelog matches this: LUCENE-10480, "Implement Block-Max-Maxscore scorer for 2 clauses disjunction", Lucene 9.3.0 (2022-07-29). — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
- **Block-max MAXSCORE that turns into a conjunction:**
  - As the threshold rises, Lucene's block-max MAXSCORE now also intersects terms. Example: for "the quick fox" with max impacts 0.2/0.5/1, a threshold in (1.2, 1.5] becomes `(the) +quick +fox`.
  - Released in Lucene 9.9: OrHighHigh about +11% and OrHighMed about +6% on nightly benchmarks. — [Elastic Search Labs](https://www.elastic.co/search-labs/blog/more-skipping-with-bm-maxscore)
  - Changelog entry GITHUB#12589 (9.9.0, 2023-12-04): "Disjunctions now sometimes run as conjunctions when the minimum competitive score requires multiple clauses to match." — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
- **BS1 + MAXSCORE (Elastic, Sept 11, 2023; Lucene 9.8, PR #12444):**
  - Test: 10M English Wikipedia docs, disjunctions of 2–24 high-frequency terms (df 400K–4M).
  - Existing top-k pruning beat exhaustive evaluation only up to about 8 terms, and was 2.5× slower than exhaustive at 24 terms.
  - Combining BS1 windowing with MAXSCORE "consistently outperformed" both exhaustive evaluation and MAXSCORE/WAND.
  - Some ELSER (learned sparse) queries were up to 5× slower with WAND than with BS1. — [Elastic blog](https://www.elastic.co/blog/speedups-top-k-queries-high-frequency-terms)
- **Original Lucene BMW (Lucene 8.0 / ES 7.0, 2019):**
  - Lucene indexes (term frequency, document length) pairs rather than precomputed scores, so impacts work with any similarity.
  - Term queries became 3–7× faster and disjunctions up to 15× faster; some disjunctions were about 8% slower.
  - This forced two changes: scores can no longer be negative, and `hits.total` is no longer always exact (`track_total_hits`). — [Elastic blog 2019](https://www.elastic.co/blog/faster-retrieval-of-top-hits-in-elasticsearch-with-block-max-wand)
  - Changelog: LUCENE-4198 "Codecs now have the ability to index score impacts" (8.0.0, 2019-03-14). — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
- **Conjunctive (block-max AND / ranked AND) in Lucene:**
  - LUCENE-8770: BlockMaxConjunctionScorer leverages two-phase iterators (8.2.0, 2019-07-26).
  - GITHUB#12719: top-level conjunctions not sorted by score get a specialized bulk scorer (9.9.0).
  - GITHUB#13994: faster top-k for filtered conjunctions (10.1.0).
  - GITHUB#15004: all iterators wrapped with likelyImpactsEnum under `BlockMaxConjunctionBulkScorer` (10.3.0). — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
  - `BlockMaxConjunctionBulkScorer` uses windows of up to 65,536 docs (`MAX_WINDOW_SIZE`). — [BlockMaxConjunctionBulkScorer.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/BlockMaxConjunctionBulkScorer.java)
- **Lucene's vectorized block-max AND (shipped in 10.3):**
  - Windows are aligned with the leading clause's impact blocks.
  - `advanceShallow` and `getMaxScore` give cumulative upper bounds, and windows that cannot beat the threshold are skipped.
  - The lead clause returns a batch via `nextDocsAndScores`, and scores are computed *before* checking other clauses (the "score-first" order). Branchless SIMD `filterByScore` prunes the batch. Remaining clauses are still checked with `advance`.
  - Lucene runs plain DAAT for the first k matches, then switches to the vectorized path.
  - Excluding a hardware change, about 40% speedup on nightly AndHighHigh. — [jpountz blog, Aug 2025](https://jpountz.github.io/2025/08/28/compiled-vs-vectorized-search-engine-edition.html)
- **Lucene's vectorized block-max MAXSCORE (Oct 11, 2025):**
  - Outer windows come from block-max data. Inner windows are capped at 4,096 docs, each with a bitset of matches and a float score array.
  - With one essential clause, batches come from `nextDocsAndScores`. With several, scores are accumulated term-at-a-time into the bitset and array.
  - Each non-essential clause gets a `minRequiredScore` (threshold minus the remaining clauses' max scores) and a branchless filter.
  - Partitions are recomputed only when the threshold rises.
  - Positioned as strong on tail latency, especially for many-term or high-frequency-only queries. — [jpountz blog, Oct 2025](https://jpountz.github.io/2025/10/11/vectorized-evaluation-of-disjunctive-queries.html)
- **"Self-optimizing query plans" (Grand, June 4, 2026):** Lucene's rewritten BM25 retrieval aims to "skip nearly as effectively as block-max WAND while evaluating hits nearly as efficiently as exhaustive evaluation". No figures are given. — [jpountz blog, June 2026](https://jpountz.github.io/2026/06/04/an-ode-to-self-optimizing-query-plans.html)
- **Hard query shapes (Grand, May 12, 2025):**
  - In "the incredibles" (stop word plus rare term), pruning kicks in late. No engine computes the top-k threshold up front.
  - In "to be or not to be" (all stop words), Tantivy and PISA were slower with TOP_100 than with TOP_100_COUNT, i.e. pruning overhead exceeded its savings. Lucene rewrites the query to `to^2 be^2 or not`, going from 6 postings lists to 4.
  - On a long paragraph query, Lucene beat Tantivy and PISA, which the author attributes to MAXSCORE vs. BMW. — [jpountz blog, May 2025](https://jpountz.github.io/2025/05/12/analysis-of-Search-Benchmark-the-Game.html)

### Inferences
- Lucene's evolution and the ECIR 2019 results point the same way. (B/V)BMW is best for short BM25 queries at small k. Block-max MaxScore with window-based batch evaluation is the more robust default for mixed production workloads: long queries, high k, counting, filters and learned-sparse weights.
- Recomputing the MaxScore partition per block, or only when the threshold rises, and turning disjunctions into conjunctions once several clauses are required brings much of WAND's skipping into MaxScore without WAND's per-document cursor sorting.

### Gaps
- The full numeric tables of Ding & Suel (SIGIR 2011, BMW and ranked AND) and of VBMW (SIGIR 2017) were not retrieved; only the abstract-level "≈2×" for VBMW is cited.
- The ECIR 2019 Table 3 latencies could not be reliably attributed to rows and columns from the PDF text extraction, so only the paper's stated conclusions are cited.
- No 2024–2026 peer-reviewed BM25 study comparing Lucene's vectorized BMM against PISA's VBMW was found. The search-benchmark-game (Q4) is the closest public comparison.

## Q3. Learned sparse retrieval (SPLADE, uniCOIL, DeepImpact): how it changed posting-list workloads, and the 2022–2026 algorithms designed for it

### Takeaway
Learned sparse retrieval (LSR) gives long queries (SPLADEv2 averages about 25 unique terms), small vocabularies with very long lists, and non-Zipfian, "wacky" weights. Classic MaxScore and BMW slow down 10–50× relative to BM25. The field moved to:
- approximate SAAT;
- index-side decompositions (clipping, guided traversal);
- docID-range block filtering with forward-index scoring (BMP, 2024; superblocks SP 2025, LSP 2026);
- clustered, statically pruned inverted lists with summary vectors (Seismic, 2024).

Rank-safe SPLADE retrieval on MS MARCO dropped from about 100–600 ms (PISA MaxScore/BMW) to about 2–10 ms (BMP/SP). Approximate retrieval at 90–99% recall is now sub-millisecond. Seismic reached production in OpenSearch 3.3 (2025).

### Cited Findings
- **Workload shift.** Learned sparse indexes differ through "sub-word tokenization, leading to longer queries, smaller vocabularies, and different score distributions within posting lists". — [Mallia, Suel, Tonellotto, SIGIR 2024](https://arxiv.org/html/2405.01117v1)
- WAND and MaxScore assume non-negative values, short queries and Zipfian list lengths, and LSR violates these assumptions, which weakens pruning. — [Bruch et al., Seismic, SIGIR 2024](https://arxiv.org/html/2404.18812v1)
- **Wacky weights (MS MARCO dev, Xeon Gold 6144, single thread) — mean latency / RR@10:**

  | Engine | BM25 | DeepImpact | uniCOIL-T5 | SPLADEv2 |
  |---|---|---|---|---|
  | Lucene (BMW) | 40.1 ms | 244.1 ms | 222.3 ms | 2,140 ms |
  | PISA MaxScore | 8.3 ms | 19.4 ms | 36.9 ms | 220.3 ms |
  | JASS exact (SaaT) | 15.8 ms | 39.3 ms | 147.2 ms | 314.0 ms |
  | JASS approx., ρ = 1M postings | — | — | 14.9 ms | 15.3 ms (RR@10 0.324 vs 0.369) |

  - SPLADEv2 queries average about 25 unique terms, versus about 4–7 for the other models.
  - SPLADEv2 is about 50× slower than BM25 in Lucene and about 25× slower in PISA.
  - In PISA on SPLADEv2, WAND and BMW took 619 and 681 ms, versus 553 ms for exhaustive OR. — [arXiv 2110.11540v2](https://arxiv.org/html/2110.11540v2)
- **Postings clipping / term impact decomposition (Mackenzie, Mallia, Moffat, Petri; Findings of EMNLP 2022):** splits postings by impact to accelerate top-k retrieval "by up to 9.6X without any loss in effectiveness". — [ACL Anthology](https://preview.aclanthology.org/new-sigs/2022.findings-emnlp.205/)
- **Guided traversal (Mallia, Mackenzie, Suel, Tonellotto; SIGIR 2022):**
  - BM25 scores drive index traversal and skipping, while DeepImpact scores the documents.
  - Both impacts are packed into one 32-bit posting value, and one top-k heap is kept per model. — [arXiv 2204.11314](https://www.arxiv.org/pdf/2204.11314)
  - Follow-up (Qiao et al., WWW 2023) extends this to SPLADE/uniCOIL. It notes that unconstrained BM25-guided skipping can visibly hurt relevance when BM25 is poorly aligned with the learned model or k is small. — [arXiv 2305.01203](https://ar5iv.labs.arxiv.org/html/2305.01203)
- **Block-Max Pruning (BMP; Mallia, Suel, Tonellotto; SIGIR 2024 short, pp. 2411–2415):** — [arXiv 2405.01117](https://arxiv.org/html/2405.01117v1)
  - The docID space is split into blocks of b consecutive docIDs (b = 8–256), with BP reordering.
  - Per-term block-max arrays are weighted by the query term weights and summed (SIMD) to give block upper bounds.
  - Only blocks above a quantile-based threshold estimate are partially sorted (counting sort).
  - Blocks are evaluated in bound order with b accumulators, using a hybrid inverted/forward index.
  - Safe stopping happens when the heap threshold exceeds the next block's bound. α gives earlier approximate stopping, and β prunes query terms.
  - Rust implementation, available through PyTerrier. — [PyTerrier BMP docs](https://pyterrier.readthedocs.io/en/latest/ext/bmp/api.html)
- **BMP safe retrieval, k=10, MS MARCO passage (Xeon 2.8 GHz):** BMP is "2.9 to 7.5 times faster than its closest competitor" on SPLADE and at least 2× faster on the others.

  | Method | SPLADE | ESPLADE | uniCOIL |
  |---|---|---|---|
  | MaxScore (PISA) | 120.6 ms | 13.6 ms | 14.5 ms |
  | BMW (PISA) | 614.2 ms | 10.8 ms | 12.4 ms |
  | IOQP | 79.1 ms | 27.3 ms | 34.8 ms |
  | Anytime | 80.6 ms | 8.2 ms | 8.3 ms |
  | Clipping | 245.9 ms | 9.8 ms | 9.1 ms |
  | BMP (b=32) | 10.5 ms | 2.3 ms | 2.5 ms |

  Caveat: the BMP rows of Table 2 were extracted from HTML and should be re-checked against the PDF. — [arXiv 2405.01117](https://arxiv.org/html/2405.01117v1)
- **BMP approximate retrieval, k=10:**
  - On SPLADE, BMP "raw" with b=64 and α=0.75 reaches 2.7 ms at RR@10 37.45 (exhaustive: 38.11).
  - IOQP at 1% reaches 1.4 ms but only RR@10 23.96. — [arXiv 2405.01117](https://arxiv.org/html/2405.01117v1)
- **Seismic (Bruch, Nardini, Rulli, Venturini; SIGIR 2024):** — [arXiv 2404.18812](https://arxiv.org/html/2404.18812v1)
  - Index structure: each posting list is statically pruned to its top-λ entries, then clustered (shallow k-means) into blocks. Each block gets a summary vector of per-coordinate maxima, α-mass pruned and 8-bit quantized.
  - A forward index of full document vectors is used for exact inner products.
  - Queries are processed coordinate-at-a-time over the top `cut` query terms. A block is skipped if its summary score is below the heap minimum divided by `heap_factor`.
  - At 90% accuracy on MS MARCO it runs at 187/222/115 µs per query (SPLADE/E-SPLADE/uniCOIL-T5).
  - It is ≥84.6×, 24.9× and 143.3× faster than IOQP; 2.6–21.6× faster than the BigANN 2023 graph winners (GrassRMA, PyANNs); and builds in 5 minutes vs. 137–267 minutes.
  - Exact PISA takes about 100 ms on SPLADE.
- **SeismicWave (CIKM 2024):**
  - Visits blocks in descending summary-score order and expands results with an offline κ-NN graph.
  - On SPLADE it is up to 2.2× faster than Seismic, e.g. 96% accuracy in 237 µs vs. 531 µs.
  - At 99% accuracy it takes 409 µs, vs. 95,818 µs for exact PISA. — [arXiv 2408.04443](https://arxiv.org/html/2408.04443v1)
- **Scale (Bruch et al., arXiv Jan 2025):** on MS MARCO v2 (about 138M SPLADE vectors), Seismic is 3.0–6.9× faster than sparse HNSW (kANNolo) at 97–98% accuracy@10. HNSW builds about 25× slower (≈10–12.5 h vs. 0.5 h without the κ-NN graph). Seismic's latency scales less favorably with collection size than HNSW's. — [arXiv 2501.11628](https://arxiv.org/html/2501.11628v1)
- **Dynamic Superblock Pruning (SP; Carlson, Xie, He, Yang; SIGIR 2025):** — [arXiv 2504.17045](https://arxiv.org/html/2504.17045v1)
  - Groups c (16–128) consecutive blocks of b (8 or 16) docs into superblocks.
  - Prunes a superblock using both its max bound (≤ θ/μ) and its average bound (≤ θ/η) before visiting child blocks.
  - SPLADE, k=10, i7-1260P: rank-safe 2.15 ms vs. 2.70 (BMP), 7.19 (ASC) and 75.7 (MaxScore).
  - At 99% recall: 0.629 ms vs. 1.44 (BMP) and 2.06 (Seismic).
  - At k=1000, rank-safe: 10.5 ms vs. 13.9 (BMP) and 124 (MaxScore).
  - Below a 99% recall budget, Seismic becomes more competitive thanks to static pruning.
  - Index sizes (uncompressed): BMP up to 37 GB, SP up to 39 GB, Seismic 13 GB.
- **Lightweight Superblock Pruning (LSP; Carlson, Xie, Shah, Yang; SIGIR 2026, Melbourne):** — [SIGIR'26 paper PDF](https://sites.cs.ucsb.edu/~tyang/papers/2026SIGIR-LSP.pdf)
  - A simpler superblock pruning rule, plus a compact index and a zero-shot configuration.
  - Simple LSP is "1.8–17x faster than SP, and 1.8–12x faster than BMP" and 2× faster than SeismicWave on BEIR zero-shot.
  - On MS MARCO it is up to 4.8× faster than SeismicWave at k=1000 but 30% slower at k=10.
  - Related preprint: [arXiv 2602.02883](https://arxiv.org/html/2602.02883v1).
- **OpenSearch 3.3 (blog updated 2025-10-28): Seismic-based "neural sparse ANN" on `sparse_vector` fields.**
  - Uses an inverted index plus a forward index. Posting lists are pruned to the top documents and clustered with summaries, and pruning happens at both token and cluster level.
  - Parameters: `n_postings`, `cluster_ratio`, `heap_factor`, `top_n`, `k`, `approximate_threshold`.
  - On 1.29B C4 documents (15 r7g.12xlarge data nodes):
    - Seismic: avg 11.77 ms, P99 27 ms, 158.7 op/s, recall@10 90.2%.
    - Standard neural sparse: 125.12 ms, P99 397 ms, 26.35 op/s.
    - Two-phase neural sparse: 45.62 ms.
    - BM25: 41.52 ms.
  - Costs: about 1 GB of memory per 1M documents; force-merge took about 3 h. — [OpenSearch blog](https://opensearch.org/?p=7662); [OpenSearch 3.3 docs](https://docs.opensearch.org/3.3/mappings/supported-field-types/sparse-vector)
- **Elasticsearch:**
  - The `sparse_vector` query (from 8.15) is "subject to performance penalties from huge boolean queries". The mitigation is query-time token pruning (`prune: true`; `tokens_freq_ratio_threshold`, `tokens_weight_threshold`), with optional rescoring of pruned tokens (`only_score_pruned_tokens`). — [Elastic Search Labs](https://www.elastic.co/es/search-labs/blog/elasticsearch-sparse-vector-query)
  - At the Lucene level, LSR queries run as disjunctions through the same MaxScore/BS1 machinery (see the ELSER 5× WAND-vs-BS1 finding in Q2). Lucene 10.3 made `FeatureField` scorer computations "more reliably vectorized" (GITHUB#15117). — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)

### Inferences
- The decisive LSR techniques abandon per-term, list-wise upper bounds. They use docID-range (BMP/SP/LSP) or cluster-level (Seismic) upper bounds computed over *all* query terms at once, followed by dense, forward-index-style scoring inside the chosen ranges. A union over 25+ long lists becomes "select a few blocks, then score them densely".
- Rank-safe methods (BMP, SP) and approximate clustered methods (Seismic) trade off by recall target and k. SP/LSP lead at ≥99% recall and at k=1000; Seismic is strongest at lower recall budgets and small k.
- Production adoption is uneven as of Oct 2026. OpenSearch ships a Seismic-style ANN. Elasticsearch relies on token pruning plus Lucene's vectorized MaxScore. No evidence was found of a BMP/SP-style block-filtering index inside Lucene itself.

### Gaps
- Two-Step SPLADE (Lassance et al., ECIR 2024) and Mackenzie, Trotman & Lin's TOIS 2023 study of DAAT/SAAT for LSR were not retrieved, so their numbers are missing.
- A TOIS journal version of Seismic was not confirmed.
- Guided-traversal speedup figures were not retrieved.
- Nightly Elastic benchmark numbers for `sparse_vector` pruning were not retrieved.
- No published head-to-head of OpenSearch's Seismic vs. Elasticsearch's pruned `sparse_vector` was found.

## Q4. Production and open-source engines: how Lucene/Elasticsearch/OpenSearch, Tantivy, Vespa, PISA, iresearch, Terrier/Anserini implement conjunctions and disjunctions over compressed postings, plus benchmarks

### Takeaway
Lucene changed substantially in 2023–2026:
- Block-max MAXSCORE with BS1-style 4,096-doc windows (9.8–9.9).
- Two-level inlined skip data (9.12).
- Bitset-encoded dense postings blocks (10.2).
- Term-at-a-time vectorized scoring windows (10.3).
- 256-doc postings blocks (10.4).

Its author reports Lucene 10.3 is on par with Tantivy on most queries. In the current search-benchmark-game data (Lucene 10.4.0, Tantivy 0.26, PISA 0.8.2, iresearch 26.03.1), Lucene (especially with BP reordering) has the lowest mean latency for top-10 unions and intersections. PISA and iresearch remain competitive on deep-k and count-plus-top-k unions.

### Cited Findings
**Lucene postings format and skip data (data structures that the AND/OR scorers rely on):**
- **Lucene 9.12.0 (2024-09-28), GITHUB#13585:** Lucene912PostingsFormat has "only 2 levels of skip data, which are inlined into postings instead of being stored at the end", giving "better performance for queries that need skipping such as conjunctions". — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
  - Grand reports a 33% speedup on CountAndHighMed, mostly from simpler skipping. Lucene previously maintained up to 10 skip levels, while Tantivy used one. — [jpountz blog, Apr 2025](https://jpountz.github.io/2025/04/12/why-is-Tantivy-faster-than-Lucene.html)
- **Lucene103PostingsFormat (10.3.1 javadoc):**
  - 128-integer packed blocks. Patching is used for frequency blocks but not docID-delta blocks.
  - Level-0 skip data every block; level-1 every 32 blocks (4,096 docs).
  - Impacts are stored as competitive (freq, norm) pairs, and only if frequencies are indexed. — [Lucene103PostingsFormat](https://lucene.apache.org/core/10_3_1/core/org/apache/lucene/codecs/lucene103/Lucene103PostingsFormat.html)
- **Dense blocks as bitsets:**
  - Lucene 10.2.0 (2025-04-10), GITHUB#14133: "Dense blocks of postings are now encoded as bit sets."
  - In the same release: `DocIdSetIterator#intoBitSet`/`loadIntoBitSet` speeds up dense conjunctions (GITHUB#14069, #14080, #14293).
  - Also in 10.2.0: `docIDRunEnd` lets conjunctions ignore clauses with long runs of matches (GITHUB#14312). — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
  - Grand reports +35% on CountOrHighHigh from the JIT-friendly bitset loading refactor (PR 14069). An earlier `popcnt` counting change (PR 12415) gave +60%. — [jpountz blog, Apr 2025](https://jpountz.github.io/2025/04/12/why-is-Tantivy-faster-than-Lucene.html)
- **Lucene 10.4.0, GITHUB#15160:** "Increased the size used for blocks of postings from 128 to 256. This gives a noticeable speedup to many queries."
  - GITHUB#15151 in the same release: impact scores computed per postings block via a bulk `SimScorer#score` API. — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
  - The benchmark-game's Lucene 10.4.0 description: bitsets for dense blocks, FOR-delta otherwise, PFOR for frequencies, block size 256, and two skip levels every 256 and 8,192 docs inlined in postings. Impacts are "decomposed into pareto-optimal (freq, norm) pairs". — [search-benchmark-game](https://github.com/quickwit-oss/search-benchmark-game)

**Lucene scorer changes 2023–2025 (all by Adrien Grand unless noted):**
- **9.8.0 (2023-09-28):** faster top-level disjunctions (GITHUB#12361, #12444, the MaxScore + BS1 combination). BP doc-ID reordering support (GITHUB#12489), "especially [faster] conjunctions". — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
- **10.1.0 (2024-12-20):** — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
  - MaxScoreBulkScorer recomputes partitions when the threshold allows a better split (#13800).
  - BooleanScorer evaluates clauses through Scorer (#13931).
  - WANDScorer computes scores on the fly to avoid advancing "tail" clauses (#14021).
  - Filtered disjunctions go through MaxScoreBulkScorer (#14014).
  - Faster top-k for filtered disjunctions and conjunctions (#14000, #13994).
  - Call sites capped at bimorphic (#14023). Grand reports +25% OrHighHigh and +30% AndHighHigh at top-100. — [jpountz blog, Apr 2025](https://jpountz.github.io/2025/04/12/why-is-Tantivy-faster-than-Lucene.html)
- **10.3.0 (2025-09-13):** — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
  - Term-at-a-time scoring windows (#14701).
  - `nextDocsAndScores` (#14709).
  - Branchless and vectorized `filterCompetitiveHits` (#14906, #14896; Ge Song, Adrien Grand).
  - Faster `nextPostings` on bitset-encoded blocks (#14935; Guo Feng).
  - "Score computations are now more reliably vectorized" (#15039).
- **Aggregate effect:** Elastic's 2025 Lucene wrap-up (Trent and Hegarty, Jan 28, 2026) reports, excluding a hardware change, "almost a 60% increase in query speed during 2025, from under 100 qps to over 170 qps" on Mike McCandless's nightly benchmarks. — [Elastic Search Labs](https://www.elastic.co/search-labs/blog/apache-lucene-wrapped-2025)

**Lucene/Elasticsearch filter (unscored Boolean) machinery:**
- `DocIdSetBuilder` buffers sorted int arrays until `maxDoc >>> 7` (1/128 of the segment) docs, then upgrades to a `FixedBitSet`. — [DocIdSetBuilder.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/util/DocIdSetBuilder.java)
- Multi-term queries with ≤16 terms (`BOOLEAN_REWRITE_TERM_COUNT_THRESHOLD = 16`) rewrite into a Boolean disjunction (heap merge). Larger term sets are accumulated into a doc-id set. — [AbstractMultiTermQueryConstantScoreWrapper.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/AbstractMultiTermQueryConstantScoreWrapper.java)
- `CONSTANT_SCORE_BLENDED_REWRITE` became the default for prefix, wildcard and range queries in 9.6.0 (GITHUB#12055), for "better skipping support". — [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
- `LRUQueryCache` (used by Elasticsearch's filter cache) caches a filter as `RoaringDocIdSet` if its density is below 1%, otherwise as a `FixedBitSet`. — [LRUQueryCache.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/LRUQueryCache.java)

**Tantivy (Rust; current source):**
- Posting blocks are 128 docs (`COMPRESSION_BLOCK_SIZE = BitPacker4x::BLOCK_LEN`). There is one skip level with block-max info, from which `block_max_score` is derived. — [tantivy skip.rs](https://github.com/quickwit-oss/tantivy/blob/main/src/postings/skip.rs)
- Top-k pruning dispatches pure-term unions to `block_wand`, if the score combiner sums, and pure-term intersections to `block_wand_intersection`. Other shapes fall back to generic scorers. — [tantivy boolean_weight.rs](https://github.com/quickwit-oss/tantivy/blob/main/src/query/boolean_query/boolean_weight.rs)
- `BufferedUnionScorer` uses a sliding window ("HORIZON") of 64×64 = 4,096 docs: 64 TinySet 64-bit buckets plus a 4,096-entry score-combiner array (a BS1-style union). — [tantivy buffered_union.rs](https://github.com/quickwit-oss/tantivy/blob/main/src/query/union/buffered_union.rs)
- Intersections order scorers by estimated cost and specialize when the two shortest are TermScorers. — [tantivy intersection.rs](https://github.com/quickwit-oss/tantivy/blob/main/src/query/intersection.rs)
- The benchmark-game description: "Impacts are inlined into skip data as an approximation only the optimal (term_freq, norm) pair for the block is stored". — [search-benchmark-game](https://github.com/quickwit-oss/search-benchmark-game)

**PISA (C++):**
- Benchmark-game description: DAAT; FastPFOR with 128-doc blocks; block-max WAND / block-max AND for top-k; impacts stored directly, which requires a single segment; BP reordering. — [search-benchmark-game](https://github.com/quickwit-oss/search-benchmark-game)
- PISA's speed comes partly from avoiding virtual calls (term-only clauses) and partly from recursive graph bisection. — [jpountz blog, Aug 2025](https://jpountz.github.io/2025/08/28/compiled-vs-vectorized-search-engine-edition.html)
- It is the reference platform for the ECIR 2019 study and most LSR baselines above. — [Mallia et al. ECIR 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

**iresearch (C++, SereneDB):** benchmark-game description: 128-doc blocks (simdcomp, streamvbyte, bitsets); a skip list every 32 blocks; block MaxScore for OR and block WAND for top-k; bitset windows of 4,096 docs for disjunctions and some conjunctions. — [search-benchmark-game](https://github.com/quickwit-oss/search-benchmark-game)

**Vespa:**
- Two WAND operators. `wand` over a weightedset field is guaranteed to return the exact top-k by inner dot product. `weakAnd` over text fields is an "approximative WAND" with `targetHits` per content node (default 100); it shares score thresholds across threads.
- Inner scoring must be a linear sum of per-term contributions.
- Example: on 8.8M MS MARCO passages, OR fully ranks 7.93M docs, while WAND with totalTargetHits=1000 fully ranks 196,900 docs and returns the same results. — [Vespa WAND docs](https://docs.vespa.ai/en/ranking/wand.html)
- For text indexes, Vespa stores a bitvector in addition to the posting list for frequent terms. Filter-only terms use the bitvector instead of the posting list. This was extended to `fast-search` attributes in 2022. — [Vespa newsletter Oct 2022](https://blog.vespa.ai/vespa-newsletter-october-2022/)
- The query planner uses posting-list hit estimates to evaluate restrictive terms first. — [Vespa serving tuning](https://docs.vespa.ai/en/performance/feature-tuning.html)

**Terrier/Anserini:**
- Anserini is Lucene-based. In the wacky-weights study the "Lucene (BMW)" rows are Anserini-style Lucene indexes, at 40.1 ms BM25 vs. 8.3 ms PISA MaxScore on MS MARCO (2021-era Lucene). — [arXiv 2110.11540v2](https://arxiv.org/html/2110.11540v2)
- PyTerrier exposes BMP through an extension. — [PyTerrier BMP](https://pyterrier.readthedocs.io/en/latest/ext/bmp/api.html)

**Search-benchmark-game, current `results.json`:**
- Setup: English Wikipedia, AOL-derived queries (≥2 terms), single thread, best of 10 runs; AMD Ryzen 7 9800X3D, Rust 1.96, OpenJDK 21.0.11; engines tantivy-0.26, lucene-10.4.0, lucene-10.4.0-bp, pisa-0.8.2, iresearch-26.03.1.
- The aggregation below was computed by this researcher: mean of per-query best-run latency in µs, over 300 intersection and 301 union queries.

  | Workload | Tantivy | Lucene | Lucene-BP | PISA | iresearch |
  |---|---|---|---|---|---|
  | TOP_10 union | 792 | 309 | 242 | 587 | 372 |
  | TOP_10 intersection | 486 | 241 | 181 | 354 | 390 |
  | TOP_1000 union | 2,258 | 1,436 | 1,022 | 1,126 | 1,166 |
  | TOP_1000 union (median) | 1,077 | 1,042 | 681 | 313 | 730 |
  | COUNT union | 936 | 364 | 368 | 3,002 | 303 |
  | COUNT intersection | 376 | 295 | 187 | 311 | 283 |
  | TOP_100_COUNT union | 3,253 | 5,206 | 4,658 | 2,948 | 1,480 |

  — [search-benchmark-game repo/results.json](https://github.com/quickwit-oss/search-benchmark-game); [results viewer](https://tantivy-search.github.io/bench/)
- The repo forbids marketing claims based on single tests. An earlier README version stated results were generated on an AWS c7i.2xlarge (AVX-512), and Grand's May 2025 analysis also used c7i; the current README lists the Ryzen machine. — [search-benchmark-game README](https://github.com/quickwit-oss/search-benchmark-game); [jpountz May 2025](https://jpountz.github.io/2025/05/12/analysis-of-Search-Benchmark-the-Game.html)
- **Grand's explanations:**
  - "Lucene 10.3 is on par with Tantivy on most queries and faster on some". At 10.2, Lucene was still slower on phrase queries and on exhaustive scored evaluation (TOP_100_COUNT). — [jpountz Apr 2025](https://jpountz.github.io/2025/04/12/why-is-Tantivy-faster-than-Lucene.html)
  - Dense postings stored as bitsets turn loading matches into bitset OR/AND, and counting into "less than one instruction per match". Tantivy's COUNT used a popcount over windowed bitsets, which Lucene adopted in 9.8. — [jpountz May 2025](https://jpountz.github.io/2025/05/12/analysis-of-Search-Benchmark-the-Game.html)

### Inferences
- The convergence point for production unions is: block-max MaxScore (or BMW) for choosing windows, plus 4,096-doc bitset/score-array windows inside them. Lucene, Tantivy and iresearch all use 4,096-doc windows. For intersections it is: block-max AND with lead-clause batches and a SIMD threshold filter. Dense postings stored as bitsets make dense ANDs and ORs word-parallel.
- In the benchmark game, BP reordering gives Lucene a further 20–30% on mean union and intersection latency (TOP_10: 309→242 µs and 241→181 µs).
- PISA's very low median for TOP_1000 unions (313 µs) suggests BMW with impacts stored directly is still very effective for deep top-k on typical queries. Its high mean on COUNT unions (3,002 µs) shows the cost of lacking bitset/popcount counting paths.
- iresearch's lowest TOP_100_COUNT union mean (1,480 µs) is consistent with its block-at-a-time design, which serves exhaustive scoring and counting well.

### Gaps
- The Lucene 10.4.0 release date was not captured; the fetched changelog shows no date for 10.4.0.
- No public Vespa lexical benchmark against Lucene or Tantivy was found. Vespa's internal posting-list encoding, its bitvector density threshold, and how `weakAnd` is implemented internally (WAND vs. MaxScore, block-max or not) were not documented in the sources retrieved.
- Elasticsearch-specific top-level behavior (e.g., `track_total_hits` defaults interacting with pruning) beyond the 2019 blog was not re-verified for ES 9.x.
- The benchmark aggregation above is this researcher's computation from the repo's raw file, not a published summary. The run date of `results.json` could not be confirmed (GitHub API access was blocked).

## Q5. Boolean (non-ranked) unions and intersections in database/analytics systems: Roaring-based OR/AND (Druid, Pinot, ClickHouse, Elasticsearch filters) and multi-way union strategies

### Takeaway
Analytics engines keep per-value posting lists as Roaring bitmaps and evaluate predicates by bitmap AND/OR. Multi-way union then becomes container-wise bitmap accumulation, usually a naive left-to-right in-place fold. A heap or priority queue is an option, but Roaring's authors find "no clear winner": the heap helps on sorted or run-heavy data, while the naive fold uses less memory and is the library default. Search engines (Lucene/Elasticsearch, Tantivy) use a heap/DAAT merge for few or sparse clauses. They switch to bitset accumulation once many clauses or dense results make a heap per document too costly (≥16 terms in Lucene's multi-term rewrite; more than 1/128 of the segment in `DocIdSetBuilder`; 4,096-doc windows in BS1-style scorers). ClickHouse (2026) adds an automatic choice between brute-force bitmap and leapfrog intersection.

### Cited Findings
- **Roaring multi-way operations (Lemire et al., "Consistently faster and smaller compressed bitmaps with Roaring", 2016):** — [arXiv 1603.06549](https://arxiv.org/pdf/1603.06549)
  - A naive pairwise fold works well for intersections, because results shrink.
  - Unions need a "lazy union" that skips cardinality maintenance in bitmap containers (bitCount can cost about 30%) and repairs the result at the end.
  - Cost model: in the naive fold N bitmaps of size B cost O(BN²) when outputs grow, versus O(BN log N) for the heap. When inputs are uncompressed bitmaps over the same range, both are O(BN), but the heap needs O(BN) memory vs. O(B).
- **Measured Roaring union results (2016 paper):**
  - On unions of 200 bitmaps there is "no clear winner between the naïve approach and the priority queue". The priority queue is preferable on WIKILEAKS; the naive fold is better on the non-sorted CENSUS datasets.
  - The priority queue is ineffective with original Roaring (without run containers).
  - Roaring+Run is 1.8× to nearly 10× faster than Concise for these unions. The authors suggest the naive approach "may be a good default". — [arXiv 1603.06549](https://arxiv.org/pdf/1603.06549)
- **RoaringBitmap Java library:** `FastAggregation.or(...)` simply calls `naive_or`. `priorityqueue_or` and `horizontal_or` ("minimizes memory usage… O(n log n)") are alternatives. `workShyAnd` intersects container keys first. — [FastAggregation.java](https://github.com/RoaringBitmap/RoaringBitmap/blob/master/roaringbitmap/src/main/java/org/roaringbitmap/FastAggregation.java)
- **Druid:**
  - `RoaringBitmapFactory.union` calls `ImmutableRoaringBitmap.or(iterator)`, i.e. `BufferFastAggregation.or`. Intersection uses `BufferFastAggregation.and`. — [Druid RoaringBitmapFactory.java](https://github.com/apache/druid/blob/master/processing/src/main/java/org/apache/druid/collections/bitmap/RoaringBitmapFactory.java)
  - A 2016 IDEAS paper (Chambi, Lemire et al.) reported up to ≈5× faster Druid analytical queries with Roaring than with Concise. — [IDEAS 2016 record](https://r-libre.teluq.ca/950) (snippet-level source; full paper not re-read)
- **Pinot:**
  - Keeps a value→bitmap inverted index for unsorted columns, and uses the sorted forward index as the inverted index for sorted columns. — [Pinot docs](https://docs.pinot.apache.org/build-with-pinot/indexing/inverted-index)
  - In `OrDocIdSet`, all index-based children (sorted ranges and bitmaps) are merged into one `MutableRoaringBitmap` by repeated `or`, a naive fold. Scan-based children are combined lazily through an `OrDocIdIterator`. — [Pinot OrDocIdSet.java](https://github.com/apache/pinot/blob/master/pinot-core/src/main/java/org/apache/pinot/core/operator/docidsets/OrDocIdSet.java)
  - `AndDocIdSet` similarly materializes the AND of index-based children into a bitmap and drives scan-based children from it, with optional "restriction pushdown". — [Pinot AndDocIdSet.java](https://github.com/apache/pinot/blob/master/pinot-core/src/main/java/org/apache/pinot/core/operator/docidsets/AndDocIdSet.java)
- **ClickHouse text index (redesign blogs Aug 2025 and Mar 24, 2026):**
  - Posting lists with more than 12 row IDs are block-partitioned Roaring bitmaps with per-block min/max for skipping. Lists of 7–12 rows use VarInt, and lists of 6 or fewer are embedded in the dictionary entry.
  - `hasAllTokens` maps to bitmap AND and `hasAnyTokens` to bitmap OR, feeding a PREWHERE virtual column.
  - Example: 0.198 s → 0.027 s on 10M rows. — [ClickHouse blog, Mar 2026](https://clickhouse.com/blog/clickhouse-full-text-search-object-storage)
- **ClickHouse settings (docs as of Oct 2026):**
  - `text_index_posting_list_apply_mode`: `materialize` (26.6) eagerly decodes posting lists into Roaring bitmaps; `lazy`, the default since 26.8, uses cursor-based on-demand decoding.
  - `text_index_postings_intersection_algorithm` (added 26.10, default `auto`) "uses leapfrog only when the sparsest list can skip whole packed blocks of the densest one", and otherwise brute-force bitmap intersection. "Intersections of 256 or more tokens always use leapfrog." — [ClickHouse text-index settings](https://clickhouse.com/docs/reference/settings/session-settings/text-index)
- **Lucene/Elasticsearch filters:** heap/disjunction for ≤16 multi-term clauses, bitset accumulation beyond that. `DocIdSetBuilder` upgrades from sorted arrays to a bitset above maxDoc/128. The query cache stores filters as Roaring if density is below 1%, otherwise as a bitset. Lucene 10.2+ stores dense postings blocks directly as bitsets. — [AbstractMultiTermQueryConstantScoreWrapper.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/AbstractMultiTermQueryConstantScoreWrapper.java); [DocIdSetBuilder.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/util/DocIdSetBuilder.java); [LRUQueryCache.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/LRUQueryCache.java); [Lucene CHANGES](https://lucene.apache.org/core/10_4_0/changes/Changes.html)
- **Exhaustive OR over many or high-frequency clauses:**
  - BS1's windowed bitset accumulation often beats the min-heap approach (BS2) for many or high-frequency clauses. — [Elastic blog, Sept 2023](https://www.elastic.co/blog/speedups-top-k-queries-high-frequency-terms)
  - Tantivy's `BufferedUnionScorer` uses the same 4,096-doc bitset-window idea. — [tantivy buffered_union.rs](https://github.com/quickwit-oss/tantivy/blob/main/src/query/union/buffered_union.rs)
- **Systems built on Roaring:** Elasticsearch, Spark, Netflix Atlas, LinkedIn Pinot, Druid, Pilosa, Hive, Tez, Kylin and others. — [Lemire et al., Roaring library paper](https://arxiv.org/pdf/1709.07821)

### Inferences
**When to use each multi-way union strategy (synthesis):**
- **Heap / DAAT k-way merge.** Best when:
  - there are few clauses or the result is sparse relative to the docID space;
  - results must be streamed lazily, e.g. into a conjunction or top-k with skipping;
  - scoring needs per-document combination together with skipping.

  Its cost grows with log(#clauses) per posting, and it cannot exploit word-level parallelism.
- **Bitmap/bitset accumulation.** Best when:
  - there are many clauses, high-frequency terms, or dense results;
  - unscored filters or counts are needed;
  - inputs are already bitmaps (Roaring containers, dense postings blocks).

  Engines bound its memory by windowing (4,096 docs) or by switching thresholds (16 terms, 1/128 density).
- **Naive fold vs. priority queue over compressed bitmaps.** The choice is data dependent. The naive in-place fold is the default in RoaringBitmap, Druid and Pinot because it keeps one growing accumulator, which becomes a dense bitmap container and allows in-place ORs.
- **Lazy vs. eager materialization.** ClickHouse's 2026 changes (lazy cursors, automatic leapfrog vs. brute-force bitmap intersection) show analytics systems adopting search-engine-style lazy iteration with skipping for selective ANDs. Bitmap materialization remains the fallback for dense cases.

### Gaps
- No recent (2023–2026) published benchmark comparing heap-based vs. bitmap-accumulation unions across Druid, Pinot and ClickHouse was found. The Roaring 2016 paper's tables are the main quantitative source.
- How ClickHouse evaluates `hasAnyTokens` (OR) in lazy mode (cursor merge vs. bitmap accumulation) is not documented in the pages retrieved.
- Druid's current filter planner (e.g., when it skips the bitmap index in favor of a scan) was not examined.
- The Druid 5× figure comes from a search snippet of the IDEAS 2016 paper and was not re-read in full.
