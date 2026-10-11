# Hardware-accelerated intersection and union of sorted integer lists / posting lists (SIMD, GPU, FPGA, PIM), as of Oct 2026

Scope note: these notes cover hardware acceleration only. Codec design, algorithmic theory and top-k dynamic pruning appear only as context. Every result has its publication year attached. Hardware generations differ widely, from Nehalem in 2011 to H100 in 2026, so speedups from different papers cannot be compared directly unless the notes say they can.

## Q1. CPU SIMD intersection of sorted lists: 2011 to 2026 state of the art

### Takeaway
SIMD intersection of 32-bit sorted lists has matured. Over a tuned scalar merge it typically gives 2-6x for similar-size lists, and up to about 12x when a precomputed bitmap/hash filter is allowed (FESIA, 2020). For heavily skewed lists the gain shrinks toward galloping/binary search. The newest practitioner benchmarks are from 2024 (SimSIMD, AVX-512 / SVE2 / NEON). They show that a software emulation of VP2INTERSECT is fastest for similar sizes, and that SVE2 or plain AVX-512 compares do best at 1:8 to 1:64 skew. I found no major new peer-reviewed SIMD sorted-set intersection algorithm from 2021-2026 beyond these.

### Cited Findings
**Schlegel, Willhalm & Lehner (ADMS@VLDB 2011): SSE4.2 STTNI**
- The method uses the SSE4.2 string-compare instruction (PCMPESTRM) to do an all-against-all compare of eight 16-bit values (or sixteen 8-bit values) per instruction. A hierarchical data layout splits 32-bit values into an upper 16-bit part and lower 16-bit parts, so wider integers also work. The paper reports "speedups up to 5.3x compared to popular fast scalar algorithms" on an Intel Core i7-920 at 2.67 GHz. It also notes that the achievable speedup requires lists of similar length. — [Schlegel et al., ADMS 2011](http://www.adms-conf.org/p1-SCHLEGEL.pdf)
- Lemire et al. (2016) re-evaluated it. A PCMPISTRM-based variant is about 15% faster than the original PCMPESTRM version. Schlegel beats a 16-bit V1 only when the length ratio is below 10; at high skew, V1 was "up to 75× faster". The Schlegel format also needs input and output in a special 16-bit partitioned layout. — [Lemire, Boytsov & Kurz, SPE 2016 (arXiv 1401.6399)](https://arxiv.org/abs/1401.6399)

**Lemire, Boytsov & Kurz (Software: Practice & Experience 46(6), 2016; arXiv 2014): V1 / V3 / SIMD Galloping**
- Hardware: Intel Xeon E5-1620 (Sandy Bridge) at 3.60 GHz, SSE2/SSE4 128-bit. — [Lemire et al. 2016](https://arxiv.org/abs/1401.6399)
- **V1** compares each element of the small list against a block of T=8 integers of the large list with SIMD equality compares. **V3** adds branch layers over blocks of 4T and binary-searches to the right T-block. **SIMD Galloping** gallops to a 32-integer block and then does a SIMD compare. — [Lemire et al. 2016](https://arxiv.org/abs/1401.6399)
- On ClusterData lists (largest list 2^22) with intersection size m/3, V1 is fastest up to a 16:1 size ratio and SIMD Galloping is fastest above 1024:1. The production heuristic uses V1 below 50:1, V3 from 50:1 to 1000:1, and SIMD Galloping at 1000:1 and above. — [Lemire et al. 2016](https://arxiv.org/abs/1401.6399)
- The SIMD procedure is "up to twice as fast" as scalar galloping for ratios up to 64:1. At larger ratios, non-SIMD galloping becomes "nearly as fast". Katsov's 2012 SSE algorithm is "only competitive ... when the lists have similar lengths." — [Lemire et al. 2016](https://arxiv.org/abs/1401.6399)
- Real queries (TREC 1M-query log, uncompressed postings, SvS), in ms/query:
  - GOV2 (sorted): SIMD SvS 0.5, galloping SvS 0.7, scalar SvS 2.8.
  - ClueWeb09: SIMD 1.5, galloping 2.8, scalar 6.6.
  - Intersection is 1.4-2x faster on docID-sorted GOV2 than on unsorted GOV2.
  — [Lemire et al. 2016, Table VI](https://arxiv.org/abs/1401.6399)
- Data distribution matters. Switching from clustered to uniform data reduced intersection speed "by up to a factor of 2.5", because clustering improves branch prediction and skipping. — [Lemire et al. 2016](https://arxiv.org/abs/1401.6399)

**Inoue, Ohara & Taura (PVLDB 8(3), Nov 2014 / VLDB 2015): branch-misprediction-reduced block merge**
- The algorithm reads blocks of 2-4 elements from each list. It resolves all-pairs comparisons with one SIMD compare and one branch. It needs no preprocessing and is a drop-in replacement for `std::set_intersection`. — [Inoue et al., PVLDB 2014](https://www.vldb.org/pvldb/vol8/p293-inoue.pdf)
- Hardware: Xeon E5-2690 (Sandy Bridge-EP) and POWER7+ at 4.1 GHz. — [Inoue et al.](https://www.vldb.org/pvldb/vol8/p293-inoue.pdf)
- Measured gains over gcc's `std::set_intersection`:
  - Up to 5.2x with SIMD and up to 2.1x without SIMD.
  - 4.8-5.2x for 32-bit integers and 4.2x for 64-bit integers.
  - 2.9x and 3.0x faster than Lemire's V1 on Xeon and POWER7+ respectively, on similar-size random arrays.
  - Block size 3 cut branch mispredictions by more than 75%.
  — [Inoue et al.](https://www.vldb.org/pvldb/vol8/p293-inoue.pdf)
- The authors say galloping-type algorithms remain better for arrays of very different sizes. — [Inoue et al.](https://www.vldb.org/pvldb/vol8/p293-inoue.pdf)

**Han, Zou & Yu (SIGMOD 2018): QFilter + BSR layout (graph workloads)**
- BSR (base + state bitmap) is a compact set layout. QFilter filters out most non-matching comparisons in one SIMD "byte-checking" step, giving intra-chunk and inter-chunk parallelism. — [PKU summary of SIGMOD 2018 paper](https://mod.icst.pku.edu.cn/english/news/publications/237805.htm)
- Profiling motivation: set intersection is 94.3% of triangle-counting time, 73.6% of maximal-clique time and 58.2% of subgraph-matching time. QFilter + SIMD + graph reordering gives "3-10x speedup" on graph algorithms. — [Lei Zou talk slides, Waterloo 2017-18](https://dsg.uwaterloo.ca/seminars/notes/2017-18/leizou_talk.pdf)
- The graph-reordering step alone improves performance by 39% on average. The reordering problem is proven strongly NP-complete. — [PKU summary](https://mod.icst.pku.edu.cn/english/news/publications/237805.htm)

**Zhang, Lu, Spampinato & Franchetti (ICDE 2020): FESIA (bitmap/hash filter + specialized SIMD kernels)**
- How it works:
  - Elements are hashed into a bitmap that is split into segments. Bitmaps are ANDed with SIMD to discard non-matching segments.
  - Specialized small all-pairs SIMD kernels, one per (size_i x size_j) segment combination, finish the job.
  - Complexity is O(n/√w + r), with w the SIMD width and r the result size.
  - Supports SSE, AVX and AVX-512.
  - The structure is built offline: about 77.7 s for the WebDocs dataset (1.7M docs, 5.27M distinct items).
  — [FESIA, ICDE 2020](https://users.ece.cmu.edu/~franzf/papers/icde2020_zhang.pdf)
- Hardware: Haswell Xeon E5-2695 (SSE/AVX) and Skylake i7-7820X (AVX-512). — [FESIA](https://users.ece.cmu.edu/~franzf/papers/icde2020_zhang.pdf)
- Measured speedups:
  - Haswell: up to 7.6x over scalar and 1.4-3.5x over other SIMD methods (Shuffling, BMiss, SIMDGalloping).
  - Skylake AVX-512: 2-4x over other SIMD methods. With 1M-element sets and varying selectivity, up to 6x over scalar and 1.4-3x over SIMD.
  - 3-way intersection: up to 4.8x over SIMD state of the art.
  - WebDocs queries: about 4x over scalar, 2x over SIMD Shuffling and 3.8x over SIMDGalloping.
  - Triangle counting: up to 12x over scalar and 1.7x over SIMD Shuffling, scaling linearly with cores.
  - The specialized AVX-512 kernels are up to 6.7x faster than a general AVX-512 kernel.
  — [FESIA](https://users.ece.cmu.edu/~franzf/papers/icde2020_zhang.pdf)
- Gains are larger at low selectivity. The authors note that in most real scenarios selectivity is below 10%. — [FESIA](https://users.ece.cmu.edu/~franzf/papers/icde2020_zhang.pdf)

**AVX-512 VP2INTERSECT (2019-2026)**
- VP2INTERSECTD/Q computes the intersection between doubleword/quadword vectors into a pair of mask registers. Support status:
  - Tiger Lake (client), the unofficial Alder Lake case, AMD Zen 5 and AMD Zen 6 per Wikipedia's table.
  - Not Sapphire Rapids or later P-core Xeons.
  - AVX10.1 matches the Sapphire Rapids AVX-512 set, which excludes VP2INTERSECT.
  — [Wikipedia: AVX-512](https://en.wikipedia.org/wiki/AVX-512); [Wikipedia: Zen 5](https://en.wikipedia.org/wiki/Zen_5)
- Press coverage of GCC's znver5 patch said VP2INTERSECT had previously appeared only in Tiger Lake and is "considered deprecated on Intel SKUs". HotHardware called it useful for "database searches". This is secondary press coverage, not an Intel statement. — [EVGA forum repost of Zen 5 GCC-patch news](https://forums.evga.com/AMD-Zen-5-Details-Emerge-with-GCC-Znver5-Patch-New-AVX-Instructions-Larger-Pipelines-m3619191.aspx); [HotHardware](https://hothardware.com/news/amd-zen5-gcc-patch)
- Díez-Cañas (arXiv 2112.06342, Dec 2021, revised Apr 2022) emulates the full VP2INTERSECT subset with plain AVX512F. Each emulation computes only one of the two masks, which is enough for sorted-list intersection or counting. Rotating both operands at once reduces the number of permutes, and the emulations are reported "faster than native" on the tested Intel hardware. The approach extends to 16-bit lanes, where no native instruction exists. — [Díez-Cañas, arXiv 2112.06342](https://arxiv.org/abs/2112.06342)
- A related Intel forum thread notes that the emulation wins only when one output mask is needed. — [Intel community thread](https://community.intel.com/t5/Software-Tuning-Performance/Request-for-feedback-on-VP2INTERSECT-emulation/td-p/1350054)

**ARM NEON / SVE2 and the 2024 SimSIMD benchmark (Ash Vardanian, Sept 2024)**
- Setup:
  - Sorted unique u16 arrays.
  - Sizes 128/128, 128/1024, 128/8192, 1024/1024 and 1024/8192.
  - Intersections of 1-95% of the smaller set.
  - SVE2/NEON measured on AWS Graviton 4. The x86 CPU for the AVX-512 numbers is not stated.
  — [Vardanian, "5x Faster Set Intersections: SVE2, AVX-512, & NEON" (2024)](https://ashvardanian.com/posts/simd-set-intersections-sve2-avx512/)
- Throughput in thousands of pairs/s, at 95% overlap:

  | Sizes | Serial | AVX-512BW | VP2INTERSECT emulation | NEON | SVE2 |
  |---|---|---|---|---|---|
  | 128/128 | 1,761 | 1,091 | 8,753 | 6,229 | 6,999 |
  | 1024/1024 | 222 | 134 | 1,338 | 751 | 785 |
  | 128/8192 (1:64) | 207 | 371 | 221 | 398 | 468 |

  The emulation reaches about 5-6x over serial when sizes are equal. At 1:64 skew, SVE2 (about 2.3x serial) beats the emulation. — [Vardanian 2024](https://ashvardanian.com/posts/simd-set-intersections-sve2-avx512/)
- The post says SVE2 is always at least as fast as NEON for u16, with mixed results for u32. — [Vardanian 2024](https://ashvardanian.com/posts/simd-set-intersections-sve2-avx512/)

**Memory-level parallelism for skewed intersection: "shotgun" (Lemire blog, Jan 2019)**
- Shotgun interleaves several (e.g., 4) binary searches from the small list into the large list, so cache misses overlap. Intersecting 32 integers against 100M sorted integers (Cannon Lake, Java 8) took 1.3 µs one-way vs 0.9 µs four-way, about 1.4x faster. It is used in production in CRoaring. — [Lemire blog, 2019](https://lemire.me/blog/2019/01/16/faster-intersections-between-sorted-arrays-with-shotgun/)

### Inferences
- Across 2011-2024 the measured SIMD gain over the best scalar code for similar-size 32-bit lists is fairly stable at roughly 2-6x. FESIA's larger numbers (up to 7.6x and 12x) depend on an offline-built auxiliary structure. Speedups over `std::set_intersection` (Inoue) are inflated relative to speedups over a branchless or galloping baseline.
- Wider vectors help less than expected for merge-style kernels. The 2024 SimSIMD data shows AVX-512BW below serial at 1024/1024. Roaring deliberately uses 128-bit vectors for its union (see Q2). The winners are designs that reduce branches and comparisons (all-pairs block compare, bitmap pre-filter, VP2INTERSECT-like masks), not raw lane count.
- VP2INTERSECT's hardware future is uncertain. It ships on AMD Zen 5 but is absent from current Intel server parts. Portable libraries therefore emulate it, and the emulation is at least as good on Intel.

### Gaps
- I found no peer-reviewed 2021-2026 paper that proposes a new general sorted-list SIMD intersection algorithm and beats FESIA/QFilter on standard IR benchmarks. Recent work is mostly practitioner benchmarks (SimSIMD) and graph-mining kernels.
- Native VP2INTERSECT latency/throughput on Zen 5 vs the emulation: no measured numbers found. Whether Intel AVX10.2 / Diamond Rapids re-includes VP2INTERSECT: not confirmed by any source I reached.
- I did not retrieve QFilter's own paper experiments section (head-to-head numbers vs BMiss/V1/Shuffling). The "3-10x" figure is from the authors' talk slides and covers whole graph algorithms.
- No independent replication of SimSIMD's x86 numbers, whose CPU is unspecified.

## Q2. SIMD union/merge, Roaring kernels, and set operations on SIMD-compressed data

### Takeaway
SIMD union is done with small bitonic/odd-even merge networks built from vector min/max, adapted from Inoue & Taura's SIMD merge sort (VLDB 2015), plus a vectorized dedupe. Union gains (about 1.5x) are smaller than intersection gains (2-3x) in Roaring's measurements. For compressed postings, the dominant practice is decode-then-intersect with very fast SIMD block decoding (about 0.7 cycles/int for S4-BP128-D4) at block granularity with skipping. The truly "compressed-domain" fast path is universe partitioning (Roaring-style chunks of 2^16), where intersection becomes bitmap AND or a 16-bit SIMD array intersection. This was measured 4-51x faster than cardinality-partitioned Elias-Fano style indexes for AND queries (Pibiri 2019).

### Cited Findings
- **Roaring's SIMD kernels (Lemire et al., SPE 48(4), 2018; arXiv 1709.07821).** Hardware: Skylake i7-6700 at 3.4 GHz. — [Lemire et al., "Roaring Bitmaps: Implementation of an Optimized Software Library"](https://arxiv.org/abs/1709.07821)
  - Array-array intersection (16-bit values) uses SSE4.1 PCMPISTRM for all-against-all comparison of 8x8 16-bit blocks. Galloping is used when one array is much smaller. AVX2 has no 256-bit equivalent of PCMPISTRM.
  - Array-array union uses a SIMD merge adapted from Inoue & Taura's SIMD/cache-friendly sort (PVLDB 8(11), 2015). Merging two sorted 8x16-bit vectors takes 8 min and 8 max instructions. A vectorized compare against the vector shifted by one element removes duplicates, followed by a lookup-table shuffle. Symmetric difference uses a similar block-based design.
  - The union deliberately uses 128-bit vectors. 256-bit AVX "gains are modest" and leave more scalar tail work.
  - Bitset-bitset operations use AVX2 with a vectorized Harley-Seal popcount to keep cardinality. The gain is more than 2x when only counts are needed.
- **Measured SIMD vs scalar in CRoaring** (cycles per input element), CENSUSINC / WEATHER datasets:

  | Operation | Speedup (CENSUSINC / WEATHER) |
  |---|---|
  | Intersections | 2.0x / 2.6x |
  | Unions | 1.5x / 1.5x |
  | Differences | 1.9x / 2.5x |
  | Symmetric differences | 1.5x / 1.4x |
  | Intersection counts | 2.8x / 3.4x |
  | Intersections on ClusterData | 3.1x (8.11 → 2.58 cycles/element) |

  The SIMD optimizations are "sometimes multiplying the performance by a factor of five." — [Lemire et al. 2018, Tables 10 & 13](https://arxiv.org/abs/1709.07821)
- An uncompressed bitset is fastest on some dense datasets, with Roaring second at "sometimes half the speed" but using up to 10x less memory. On CENSUS1881, Roaring is an order of magnitude faster than any alternative for intersections. Hash sets perform worst because of poor locality. — [Lemire et al. 2018](https://arxiv.org/abs/1709.07821)
- **Decode-then-intersect with SIMD codecs (Lemire et al. 2016).**
  - S4-BP128-D4 decodes at about 0.7 cycles per integer, nearly 9 billion 32-bit ints/s with one-step unpacking at 3.6 GHz.
  - On TREC 1MQ with lists decompressed and then intersected, SIMD codecs give 2-3x faster query times than FastPFOR/VarInt at comparable compression.
  - Adding bitmaps for long lists (the HYB+M2 bitmap-list hybrid) gives up to about 4x faster queries, at up to about 2x larger index.
  - The authors validated decode-then-intersect against a skipping alternative.
  — [Lemire et al. 2016](https://arxiv.org/abs/1401.6399)
- **Universe-partitioned vs cardinality-partitioned compressed lists (Pibiri, arXiv 1907.01032, 2019).** Hardware: i7-4790K at 4.0 GHz.
  - Indexes partitioned by universe (Roaring R2/R3 and Pibiri's recursive "Slicing" S) run AND queries 6-51x faster than cardinality-partitioned ones (e.g., partitioned Elias-Fano) at density 10^-2. The range is 5-42x at 10^-3 and 4-23x at 10^-4. The cause is far fewer instructions and branches (e.g., 10x fewer instructions at d=10^-2).
  - Recompiling CRoaring with SIMD disabled made R2 perform the same as S, so "vectorization does the difference".
  - In Slicing, SIMD small-array intersection (a Schlegel-style 8-bit all-pairs compare) improves AND time by about 1.6-2.2x at d=10^-2: 816→507, 1959→1080 and 5190→2370 µs/query on Gov2/CW09/CCNews.
  - Run containers (R3) "prevent some SIMD optimizations".
  — [Pibiri, "On Slicing Sorted Integer Sequences"](https://arxiv.org/abs/1907.01032)
- **GPU compressed-domain intersection (Ao et al., PVLDB 4(8), 2011).**
  - Linear Regression Compression (LRC) and Hash Segmentation keep enough structure to binary-search inside compressed lists on GPU.
  - Intersection on HS256-LRC-compressed lists is comparable in speed to binary search on uncompressed lists, at a 0.80% compression-ratio cost.
  — [Ao et al., VLDB 2011](https://www.vldb.org/pvldb/vol4/p470-ao.pdf)
- **GPU decode-then-intersect (Griffin, 2018).** Griffin-GPU decodes Elias-Fano on GPU with a parallel "Para-EF" decoder, then intersects with merge-path or binary search. — [Liu, Wang & Swanson, Griffin (PPoPP 2018)](https://www.cs.purdue.edu/homes/csjgwang/pubs/PPoPP18_Search.pdf)

### Inferences
- "Intersect in the compressed domain" in practice means "pick a representation whose decoded form is SIMD-friendly and whose blocks can be skipped wholesale". Examples are Roaring containers, bitmaps for dense lists and 16-bit chunk arrays. Operating on bit-packed deltas without decoding is rare, because prefix-sum dependencies make it awkward. Fast SIMD decoding makes decode-then-intersect at block granularity cheap.
- Union benefits less from SIMD than intersection, because output size is about n1+n2 and is store-bound. Merge networks also add fixed shuffle overhead. Expect about 1.5x, not 2-3x.

### Gaps
- No 2023-2026 measurements found for CRoaring's newer AVX-512 code paths, or for SIMD union on ARM SVE/SVE2.
- I found no head-to-head study of "intersect directly on SIMD-BP128 blocks (block-max/skip, decode only candidate blocks)" vs full decode on modern AVX-512 CPUs. Lemire et al. 2016 is the most recent primary source located.

## Q3. GPU set intersection and GPU inverted-index / sparse-retrieval engines

### Takeaway
GPUs help list intersection mainly through batching many queries (throughput), not through single short-query latency. Early work found GPU conjunctive intersection only about 15% faster than CPU when data was already on the GPU (Ding et al. 2009). Later throughput-oriented designs report about 10-15x over a single CPU core or an optimized CPU (Ao 2011, Griffin 2018). In graph analytics (triangle counting), GPU intersection evolved from merge to binary search (TriCore 2018) to hashing (TRUST 2021). By 2026, the GPU sparse-retrieval work replaces intersection/WAND with batched scatter-add over a GPU-resident inverted index (GPUSparse, June 2026).

### Cited Findings
- **Ding, He, Yan & Suel (WWW 2009).** GPU parallel merge-find intersection was only "marginally superior to a CPU implementation (≈15% faster)" on conjunctive queries, even with data already in GPU memory. It reached 7x on disjunctive queries. — [as summarized in Lemire et al. 2016](https://arxiv.org/abs/1401.6399); [Ding et al. paper](https://research.engineering.nyu.edu/~suel/papers/gpu.pdf)
- **Wu et al. (IPDPS 2010 workshop).** CPU-GPU cooperative framework: the CPU groups queries into batches and the GPU processes each batch in parallel. — [Ao et al. 2011 related work](https://www.vldb.org/pvldb/vol4/p470-ao.pdf); [Wu et al. slides](http://www.ipdps.org/ipdps2010/ipdps2010-slides/LSPP/IPDPS_LSPP.pdf)
- **Ao et al. (PVLDB 4(8), 2011), NVIDIA GTX 480.** — [Ao et al.](https://www.vldb.org/pvldb/vol4/p470-ao.pdf)
  - Linear-regression (LR) and hash-segmentation (HS) search-range narrowing cut global-memory accesses.
  - HS16 reaches 91,382 queries/s, 60% higher than plain parallel binary search, for 9% extra memory.
  - Speedups over the fastest single-threaded CPU algorithm reach 14.67x on uncompressed lists and 8.81x on compressed lists (BD/BDR datasets).
  - Speedup scales almost linearly with the number of enabled SMs.
  - Branch divergence is a key limiter: 24-28% more divergent branches on randomized docIDs.
- **Griffin (Liu, Wang & Swanson, PPoPP 2018).** — [Griffin](https://www.cs.purdue.edu/homes/csjgwang/pubs/PPoPP18_Search.pdf)
  - Hardware: NVIDIA Tesla K20 with a 4-core Xeon.
  - GPU parallel binary search is efficient only when the list-length ratio is large. Its benefit fades below about 128:1, and more than 64% of real intersections (TREC 2005/2006 queries on ClueWeb12) had a ratio below 128. Below that ratio Griffin uses a GPU merge-path intersection.
  - A runtime scheduler moves each query's operations between CPU and GPU. Results: 10x faster than an optimized CPU-only engine, 1.5x over GPU-only, and p95/p99/p99.9 latency cut by 10.4x/16.1x/26.8x.
- **GPU triangle counting, the most active GPU set-intersection area:**
  - TriCore (SC 2018) argues that binary search suits GPUs better than merge because of parallelism and balance. It caches top search-tree levels in shared memory (18% fewer global transactions) and reports up to 24x over prior state of the art. — [TriCore SC18](https://www2.seas.gwu.edu/~howie/publications/TriCore-SC18.pdf) (figures from search-result summary; full paper not read)
  - TRUST (arXiv 2103.08053, 2021) argues hashing is the right GPU intersection primitive, with vertex-centric hash tables. On one V100 it is 50.1x faster on average than TriCore (3.4x-17.2x on most graphs, 465x on one graph). It is 7.1x and 21.3x faster than Bisson et al. and H-INDEX. It is the first to exceed one trillion TEPS, scaling to more than 1,000 GPUs. TriCore's binary search does well on power-law graphs but poorly on regular ones. — [TRUST](https://arxiv.org/abs/2103.08053)
- **GPU learned-sparse retrieval (2026).** GPUSparse (arXiv 2606.26441, June 2026; single-author preprint, MIT-IBM Watson AI Lab) uses block-aligned, warp-coalesced posting lists padded to 32 entries and batched scatter-add scoring in fused Triton kernels. — [GPUSparse](https://arxiv.org/abs/2606.26441)
  - On MS MARCO (8.8M passages, SPLADE) with an NVIDIA H100: 1.27 ms/query vs 298 ms for Pyserini CPU (235x), and 787 QPS at batch 500 with exact scoring.
  - It reaches 62.6% of H100 peak HBM bandwidth.
  - Compared with Seismic (fast approximate CPU), it is slower per query but exact: R@1000 0.983 vs 0.738.
  - This is disjunctive scoring, not intersection.

### Inferences
- The GPU literature has two intersection regimes. Skewed lists favor binary search or hash probes; similar-size lists favor merge-path or hashing. Griffin's 128:1 crossover on K20 is the only explicit IR-workload threshold found, and it is hardware-specific.
- For IR, the GPU's advantage is batch throughput and tail latency under load, plus very high HBM bandwidth (H100). Single-query conjunctive latency on short lists is dominated by launch and transfer overhead, consistent with the 2009 "≈15%" result. Recent GPU IR systems (GPUSparse) sidestep intersection entirely and score exhaustively. This works because learned-sparse queries are disjunctive.
- The very large GPU speedups (235x, 14.67x) are against single-threaded CPU baselines, so they overstate the gain over a tuned multicore SIMD CPU.

### Gaps
- No 2020-2026 peer-reviewed paper found specifically on GPU conjunctive posting-list intersection for web search with modern GPUs (A100/H100) vs modern AVX-512 multicore CPUs.
- TriCore and H-INDEX details are from summaries. I did not verify their numbers in full text.
- GPUSparse is a recent single-author preprint whose CPU baseline appears single-threaded; its results are not independently replicated.

## Q4. FPGA, processing-in-memory (UPMEM etc.), near-data processing and specialized accelerators

### Takeaway
Dedicated IR accelerators exist mainly as simulation studies. IIU (ASPLOS 2020) reports 13.8x lower latency and 5.4x higher throughput than Lucene, in cycle-level simulation. Recent near-memory work targets sparse ANN / learned sparse retrieval (SpANNS, 2026, CXL near-memory: 15.2-21.6x over CPU). Set-intersection PIM research is concentrated in graph mining: SISA (MICRO 2021), DIMMining and NDMiner (ISCA 2022), PIMMiner (2023), AceMiner (2024). On real UPMEM hardware, set-intersection-heavy triangle counting still generally loses to a GPU and often to a CPU. I found no FPGA accelerator specifically for posting-list intersection.

### Cited Findings
- **IIU (Heo et al., ASPLOS 2020).** A specialized inverted-index processing unit co-designed with the index format, so it operates directly on highly compressed indexes. Modules connected by flexible interconnects exploit intra- and inter-query parallelism. Cycle-level simulation vs Apache Lucene: on average 13.8x lower query latency, 5.4x higher throughput and 18.6x less energy. — [IIU abstract, ASPLOS 2020](https://asplos.hosting2.acm.org/wp/wp-content/uploads/2020/abstracts/paper_12_5.html)
- **SpANNS (Zhang, Ponzina & Rosing, arXiv 2601.03229, Jan 2026).**
  - Near-memory architecture for sparse-vector ANNS (SPLADE-type) on a CXL Type-2 platform.
  - A controller handles query parsing and cluster filtering. Compute-enabled DIMMs do inverted-index traversal and distance computation near the data.
  - 15.2-21.6x faster than state-of-the-art CPU baselines.
  — [SpANNS](https://arxiv.org/abs/2601.03229)
- **SISA (Besta et al., MICRO 2021).** Set-centric ISA for graph mining on PIM. — [SISA](https://arxiv.org/abs/2104.07582)
  - Dense sets (high-degree vertices) are bitvectors processed with in-DRAM bulk bitwise AND/OR (Ambit-style).
  - Sparse sets are sorted integer arrays processed in near-memory logic layers.
  - The ISA has both merge and galloping intersection variants, and a hardware controller picks the variant at runtime.
  - More than 10x speedup over Bron-Kerbosch maximal-clique listing.
- **Graph-mining PIM accelerators (authors' own claims):**
  - PIMMiner (arXiv 2306.10257, 2023): 59x over NDMiner and 2.7x over DIMMining. — [PIMMiner](https://arxiv.org/abs/2306.10257)
  - AceMiner (DAC 2024): 40.2% over NDMiner and 13.3% over DIMMining. — [AceMiner](https://grc.iit.edu/publications/yan-2024-aceminer-c390)
  - DIMMining (ISCA 2022) and NDMiner (ISCA 2022) are near-memory graph-mining designs focused on set operations. — [NDMiner PDF](https://tnm.engin.umich.edu/wp-content/uploads/sites/353/2023/03/2022.06.NDMiner.pdf)
- **Real UPMEM hardware (2025):**
  - Triangle counting on 2,560 DPUs (20 PIM DIMMs) with vertex coloring, reservoir sampling and Misra-Gries for high-degree nodes. In static counting, "the GPU implementation consistently outperforms both the CPU and PIM implementations". PIM "generally lags behind" except on one graph with high triangle count and low max degree. PIM is advantageous for dynamic graphs updated in COO format. — [Accelerating Triangle Counting with Real PIM Systems, arXiv 2505.04269](https://arxiv.org/abs/2505.04269)
  - UPMEM DPUs are 32-bit in-order RISC cores, and inter-DPU communication goes through the host. — [arXiv 2505.04269](https://arxiv.org/abs/2505.04269)
  - TriPIM (MemSys 2025) runs TriCore-style binary search per partition across UPMEM DIMMs and reports beating TriCore (GPU) and GAP (CPU) on large graphs. — [TriPIM](https://www.memsys.io/wp-content/uploads/ninja-forms/5/TriPIM_Baradaran_MemSys25-1.pdf) (from search-result summary; not verified in full text)
  - The PRISM framework (UPMEM) uses bitmap-based set intersection with asynchronous pipelining. — [PRISM GitHub mirror](https://gitblind.noratr.app/CGCL-codes/PRISM)
- **FPGA:** GraphMatch (arXiv 2402.17559, 2024) is an FPGA subgraph-query accelerator. Its "AllCompare" multi-set intersection exploits many parallel comparators, in contrast to CPU LeapFrog, which advances one element per set per loop. — [GraphMatch](https://arxiv.org/abs/2402.17559)
- **Other hardware IP:**
  - An Intel patent describes vectorized sorted-set intersection using AVX-512 conflict-detection instructions. — [US Patent 11231935](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/11231935)
  - A patent (US 11544270, filed 2020) describes a hardware accelerator that reads inverted-list blocks and computes document scores. — [US 11544270](https://patents.google.com/patent/US11544270)

### Inferences
- For text IR, specialized hardware has not displaced CPUs. IIU is simulation-only against Lucene, which is not a SIMD-optimized intersection baseline. The real-hardware PIM evidence (UPMEM) shows limits from weak DPUs and host-mediated communication. Near-memory designs are increasingly aimed at learned-sparse / ANN workloads, where bandwidth dominates, not at classic Boolean intersection.
- The hybrid design pattern recurs across platforms: bitmaps for dense sets and sorted arrays with merge/galloping for sparse sets. Examples are SISA, Roaring, Lemire's HYB+M2 and FESIA. It appears to be the hardware-agnostic consensus representation.

### Gaps
- No FPGA accelerator specifically for posting-list intersection/union was found (2015-2026). No UPMEM evaluation of inverted-index Boolean query processing was found.
- IIU's full paper (index format, area/power, which query types) was not retrieved. Only abstract-level numbers are available.
- DIMMining/NDMiner speedups come from competitor papers. I did not read the originals.

## Q5. Multi-core / parallel intersection and partitioning strategies

### Takeaway
Multi-core list intersection parallelizes well when lists are split by value range (quantile partitioning) into independent partitions sized for cache. Tsirogiannis et al. (VLDB 2009) showed near-linear scaling on 8 cores. Production search engines mostly get parallelism from inter-query concurrency and document sharding. Intra-query parallelism matters for tail latency (Griffin 2018) and is combined with per-core SIMD (FESIA scales linearly with cores).

### Cited Findings
- **Tsirogiannis, Guha & Koudas (PVLDB 2(1), VLDB 2009).** — [Tsirogiannis et al. VLDB 2009](https://vldb.org/pvldb/vol2/vldb09-pvldb37.pdf)
  - Algorithms: Dynamic Probes (adaptive probing order using a cache-resident micro-index), Quantile-based (QB, precomputed probing order) and a hash-based algorithm for unsorted lists.
  - Lists are partitioned by value quantiles into independent partitions, one per core. For unsorted lists, partitions are sized to fit 1/C of the L2 cache.
  - Hardware: dual quad-core Intel E5355 (8 cores, 2.6 GHz).
  - All algorithms "scale almost linearly as the number of cores increases".
  - With 16 anti-correlated lists, QB is 2.5x faster than Dynamic Probes and 6x faster than Small Adaptive. Linear Merge is 1.6x slower than QB for four lists.
  - For unsorted lists, hash-based beats sort-then-intersect.
- **Tatikonda, Cambazoglu & Junqueira (SIGIR 2011).** "Posting list intersection on multicore architectures" studies coarse-grained (inter-query) and fine-grained (intra-query) parallelization models. — [cited in Lemire et al. 2016 ref. 6](https://arxiv.org/abs/1401.6399); [IBM listing via search](https://researcher.draco.res.ibm.com/publications/posting-list-intersection-on-multicore-architectures) (numbers not retrieved)
- **FESIA (ICDE 2020).** Each core intersects the small set against the bitmap of the large set independently. Triangle-counting speedups "scale linearly with the number of CPU cores" (4- and 8-core runs). — [FESIA](https://users.ece.cmu.edu/~franzf/papers/icde2020_zhang.pdf)
- **Griffin (PPoPP 2018).** Intra-query parallelism across CPU and GPU, chosen at runtime per operation, cuts p99.9 latency by 26.8x vs CPU-only. — [Griffin](https://www.cs.purdue.edu/homes/csjgwang/pubs/PPoPP18_Search.pdf)
- **Graph workloads (TRUST 2021).** Hashing-based 2D partitioning sustains scaling past 1,000 GPUs, with a 1.9x speedup going from 512 to 1,024 GPUs on extremely large graphs. — [TRUST](https://arxiv.org/abs/2103.08053)

### Inferences
- Value-range (universe) partitioning is the shared enabler for multicore splits (Tsirogiannis), Roaring's 2^16 chunks, Pibiri's slicing and FESIA's segments. It creates independent, cache-sized, SIMD-friendly sub-problems with no cross-partition merge.

### Gaps
- No 2020-2026 study found that measures intra-query multicore intersection on modern many-core CPUs (e.g., 64-128 core Zen 4/5 or Sapphire/Granite Rapids) for web-search postings.
- Tatikonda et al.'s quantitative results were not retrieved.

## Q6. Consensus: fastest practical approach for (a) similar-size lists and (b) highly skewed lists

### Takeaway
No single source states a formal consensus. The measurements converge on an adaptive, size-ratio-switched strategy (Lemire 2016 thresholds; QFilter, FESIA and Roaring all switch). (a) For similar sizes, use a branch-reduced SIMD block merge or all-pairs compare, or a VP2INTERSECT-style mask kernel. If an index-time structure is allowed, use a bitmap/hash pre-filter (FESIA), or Roaring-style containers with bitmap AND for dense data. (b) For heavy skew, use galloping/binary search from the small list: SIMD galloping in the last step, interleaved "shotgun" searches for memory-level parallelism, or a hash/bitmap probe. SIMD's advantage shrinks as skew grows.

### Cited Findings
- Lemire et al. (2016) crossovers on Sandy Bridge for 32-bit lists:
  - V1 (SIMD block compare) is best up to 16:1 and used up to 50:1.
  - V3 is used from 50:1 to 1000:1.
  - SIMD galloping is used at 1000:1 and above, and is fastest beyond 1024:1.
  - Scalar galloping approaches SIMD speed at high ratios.
  — [Lemire et al. 2016](https://arxiv.org/abs/1401.6399)
- Inoue et al. (2014): the branch-reduced SIMD merge is best for similar sizes, and galloping is better for very different sizes. — [Inoue et al.](https://www.vldb.org/pvldb/vol8/p293-inoue.pdf)
- FESIA (2020): with small skew (n1 ≪ n2), the hash-probe variant FESIA_hash is best and 2-3x faster than SIMDGalloping. When the size ratio exceeds about 1/4 (sizes closer), the merge variant FESIA_merge wins. — [FESIA](https://users.ece.cmu.edu/~franzf/papers/icde2020_zhang.pdf)
- QFilter's system switches automatically between SIMD Shuffling and SIMD Galloping based on the skew ratio. — [Zou talk slides](https://dsg.uwaterloo.ca/seminars/notes/2017-18/leizou_talk.pdf)
- Roaring (2018): PCMPISTRM SIMD intersection for comparable 16-bit arrays, galloping when one array is much smaller, and bitmap AND (AVX2) for dense containers. — [Lemire et al. 2018](https://arxiv.org/abs/1709.07821)
- SimSIMD (2024):
  - Equal sizes (128/128, 1024/1024): VP2INTERSECT emulation is best, about 5-6x serial.
  - 1:8 skew: SVE2 and AVX-512BW are about tied (about 2.7x serial).
  - 1:64 skew: SVE2 is best at about 2.3x serial, and the VP2INTERSECT emulation is barely above serial.
  — [Vardanian 2024](https://ashvardanian.com/posts/simd-set-intersections-sve2-avx512/)
- Memory-bound skewed case: interleaving 4 binary searches gave about 1.4x over sequential galloping (32 vs 100M elements). — [Lemire 2019 blog](https://lemire.me/blog/2019/01/16/faster-intersections-between-sorted-arrays-with-shotgun/)
- On GPU, binary search is preferred above about 128:1 and merge-path below it (K20). — [Griffin 2018](https://www.cs.purdue.edu/homes/csjgwang/pubs/PPoPP18_Search.pdf)
- For compressed postings, universe-partitioned representations with SIMD containers beat cardinality-partitioned Elias-Fano by 4-51x on AND queries. — [Pibiri 2019](https://arxiv.org/abs/1907.01032)

### Inferences
- A practical 2026 recipe for a CPU search engine, synthesized from the above and not stated in any single source:
  1. Store long/dense lists as bitmaps or Roaring-style containers and medium lists in SIMD-decodable blocks.
  2. Choose the kernel per pair by length ratio: SIMD all-pairs/VP2INTERSECT-emulated merge below about 16-50:1, SIMD-assisted galloping or skip-pointer block search above about 1000:1, and hybrid in between.
  3. Use SvS order (smallest list first) for k-way queries.
  4. Batch or interleave probes to exploit memory-level parallelism when the long list is out of cache.
- Thresholds are hardware- and data-dependent. Sandy Bridge-era (2016) crossovers (50:1, 1000:1) and K20-era GPU crossovers (128:1) should be re-tuned on AVX-512/SVE2 machines. The 2024 SimSIMD data suggests that wide-vector merge kernels lose their edge by about 1:8 to 1:64.
- Hardware accelerators (GPU, PIM, ASIC) pay off mainly for high-throughput batched workloads, or where bandwidth dominates (learned sparse retrieval). For low-latency conjunctive queries over short lists, a well-tuned multicore SIMD CPU remains the practical default.

### Gaps
- No 2024-2026 survey or benchmark paper explicitly states a community consensus. The recommendations above are synthesized across papers using different hardware, data distributions and baselines.
- No unified benchmark found that compares FESIA, QFilter, Inoue BMiss, Lemire V1/V3/galloping, Roaring and VP2INTERSECT emulation on the same modern CPU (e.g., Zen 5 or Sapphire Rapids) with real posting lists.
