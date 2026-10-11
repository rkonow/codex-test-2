# Compressed Posting-List Encodings and Their Support for Intersection/Union (state as of Oct 2026)

Scope note: these notes cover compression formats and how they interact with set operations (NextGEQ/skip, random access, decode speed, operating directly on compressed form). Ranked dynamic pruning (WAND/BMW/MaxScore), SIMD kernels on uncompressed data, and GPU/FPGA work are mentioned only as context. Each result is dated by its publication year. Where text was extracted from PDFs/HTML with layout damage, this is flagged.

## What do the major comparative studies conclude?

### Takeaway
No single codec wins every metric. Universe-partitioned representations (Roaring, Slicing) are the fastest for both AND and OR, roughly 4–15x faster than list codecs in Pibiri & Venturini's 2019/2021 benchmark, but they cost about 1.7–2.1x the space of partitioned Elias-Fano (PEF). Among cardinality-partitioned list codecs:
- **Space:** BIC and PEF are the most compact, and BIC is very slow.
- **Speed/space middle ground:** SIMD-BP128 matches VByte-family speed at much better compression.
- **Selective queries:** PEF is competitive when queries skip a lot (AND, BMW/VBMW).

Wang et al. (SIGMOD 2017) independently found that Roaring has the fastest intersections and SIMD-BP128-style list codecs the fastest unions and decompression.

### Cited Findings
**Pibiri & Venturini, "Techniques for Inverted Index Compression" (ACM CSUR 2021; arXiv 1908.10598, 2019)**

Setup and code:
- Benchmarks run on Gov2, ClueWeb09 and CCNews, using lists with >4096 postings:

  | | Gov2 | ClueWeb09 | CCNews |
  |---|---|---|---|
  | Integers | 5.32B | 14.86B | 19.69B |
  | Gap entropy (bits) | 3.02 | 4.46 | 5.44 |

  The full suite is released at github.com/jermp/2i_bench. — [Pibiri & Venturini (ar5iv)](https://ar5iv.labs.arxiv.org/html/1908.10598)

Space, docids, bits/int (Gov2 / ClueWeb09 / CCNews), from Table 11 — [Pibiri & Venturini](https://ar5iv.labs.arxiv.org/html/1908.10598). (Caveat: the extracted table layout was partly corrupted, so treat last digits with caution.)

| Codec | Gov2 | ClueWeb09 | CCNews |
|---|---|---|---|
| BIC | 2.94 | 4.43 | 5.24 |
| PEF | 3.11 | 4.99 | 5.45 |
| DINT | 3.53 | 5.35 | 6.44 |
| Opt-PFor | 3.63 | 5.46 | 6.07 |
| Elias δ | 3.73 | 5.17 | 6.36 |
| Opt-VByte | 3.88 | 5.72 | 6.42 |
| Rice | 4.08 | 5.31 | 5.82 |
| Simple16 | 4.18 | 5.85 | 6.41 |
| Slicing | 4.31 | 7.06 | 7.78 |
| QMX | 5.12 | 7.29 | 7.40 |
| Roaring | 6.63 | 9.78 | 9.49 |
| VByte | 8.81 | 9.20 | 9.28 |

Decoding speed:
- Roaring and Slicing decode at 0.5–0.7 ns/int. Opt-VByte, QMX and PEF take 0.7–1.3 ns/int. BIC, δ and Rice are slowest; BIC is about 2–7 ns/int (approximate, from garbled extraction).
- Prefix-summing gaps alone costs about 0.5 ns/int.
- [Pibiri & Venturini](https://ar5iv.labs.arxiv.org/html/1908.10598)

Query times, ms/query (avg). (Caveat: these are the most legible values from heavily garbled Tables 12–13.)
- **AND (Gov2 / ClueWeb09 / CCNews):**

  | Codec | Gov2 | ClueWeb09 | CCNews |
  |---|---|---|---|
  | Roaring | 0.59 | 2.87 | 2.45 |
  | Slicing | 1.03 | 4.53 | 4.30 |
  | QMX | 2.53 | 11.78 | 18.60 |
  | VByte | 2.76 | 12.50 | 19.52 |
  | PEF | 2.91 | 13.48 | 21.17 |
  | Opt-VByte | 2.98 | 13.27 | 19.76 |
  | DINT | 3.28 | 15.04 | 24.21 |
  | Opt-PFor | 3.45 | 16.25 | 23.81 |
  | Simple16 | 3.69 | 16.61 | 25.53 |
  | BIC | 10.01 | 45.30 | 72.75 |

  — [Pibiri & Venturini](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **OR:**

  | Codec | Gov2 | ClueWeb09 | CCNews |
  |---|---|---|---|
  | Roaring | 3.66 | 10.34 | 9.24 |
  | Slicing | 5.20 | 15.30 | 14.76 |
  | QMX | 52.99 | 155.19 | 168.55 |
  | VByte | 54.41 | 156.74 | 172.27 |
  | PEF | 69.77 | 207.90 | 226.21 |

  — [Pibiri & Venturini](https://ar5iv.labs.arxiv.org/html/1908.10598)

Authors' conclusions:
- "The most effective method is BIC with PEF being close second." VByte and Roaring are the least effective; Roaring is worse than VByte on ClueWeb09 and CCNews.
- PEF, DINT, Opt-VByte, Simple16, Opt-PFor and QMX form a cluster of similar tradeoffs. BIC, δ and Rice are always the slowest and are dominated.
- Roaring and Slicing "outperform in efficiency all techniques by a wide margin." Roaring is faster than Slicing because it makes better use of SIMD.
- [Pibiri & Venturini](https://ar5iv.labs.arxiv.org/html/1908.10598)

Why the two families behave differently:
- For cardinality-partitioned codecs, intersection speed tracks NextGEQ speed and union speed tracks sequential decoding speed.
- Roaring and Slicing are universe-partitioned and intersect without NextGEQ. They skip only at chunk granularity (e.g., 2^16 universe chunks), so they pay in-chunk cost even when few elements match. Cardinality-partitioned codecs can make large skips on selective queries.
- For AND, the gap between the two families narrows as the number of query terms grows. For OR, it widens.
- [Pibiri & Venturini](https://ar5iv.labs.arxiv.org/html/1908.10598)

**Mallia, Siedlaczek & Suel, ECIR 2019 (reproducibility track; PISA)**

Setup:
- Hardware: Haswell i7-4770 with AVX2.
- Collections: Gov2 (5.74B postings) and ClueWeb09 (15.86B postings).
- Docid orderings compared: Random, URL and BP (recursive graph bisection).
- All block codecs use 128-posting blocks. Implementations come from FastPFor (VarintGB, VarintG8IU, OptPFD, Simple16, Simple8b, SIMD-BP128, StreamVByte), ds2i (PEF, Interpolative), JASSv2 (QMX) and Petri's reference ANS (Packed+ANS2).
- [Mallia et al. 2019 (PDF)](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

Space, docid bits/posting on Gov2 (Random → URL → BP):

| Codec | Random | URL | BP |
|---|---|---|---|
| Packed+ANS2 | 7.71 | 3.96 | 3.25 |
| Interpolative | 7.58 | 3.80 | 3.11 |
| PEF | 7.60 | 4.11 | 3.30 |
| OptPFD | 8.13 | 4.48 | 3.74 |
| Simple16 | 9.43 | 5.34 | 4.62 |
| Simple8b | 9.24 | 5.53 | 4.77 |
| QMX | 9.16 | 5.98 | 5.19 |
| SIMD-BP128 | 8.82 | 6.35 | 5.42 |
| Varint-G8IU | 11.38 | 10.35 | 10.09 |
| VarintGB | 12.01 | 11.15 | 10.94 |
| StreamVByte | 12.30 | 11.37 | 11.21 |

— [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

Effect of reordering on size:
- Byte-aligned methods "benefit little from URL or BP reordering," while packing methods are "highly sensitive to ordering."
- With URL ordering, Packed+ANS2 shrinks Gov2 by 43% and ClueWeb09 by 27% versus Random, and BP improves further.
- [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

Query times, Gov2 TREC05, random order, k=10 (ms):

| Codec | MaxScore | VBMW |
|---|---|---|
| SIMD-BP128 | 6.27 | 5.75 |
| Varint-G8IU | 5.86 | 5.75 |
| StreamVByte | 6.44 | 6.09 |
| OptPFD | 7.43 | 7.22 |
| PEF | 11.77 | 6.23 |
| Packed+ANS2 | 13.91 | 13.73 |
| Interpolative | 16.53 | 18.65 |

— [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

Conclusions:
- SIMD-BP128 performs "on a par with the variable byte techniques while providing a significantly higher compression ratio."
- PEF "almost matches the performance of the top four encodings for algorithms utilizing many skips or lookups: BMW and especially VBMW," while cutting index size by more than 50% versus variable-byte codecs.
- Entropy-based codecs (ANS, Interpolative) are slowest.
- Queries on randomly ordered indexes can be almost 3x slower than on URL-ordered ones.
- [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

**Wang, Lin, Papakonstantinou & Swanson, "An Experimental Study of Bitmap Compression vs. Inverted List Compression" (SIGMOD 2017)**

Setup:
- Compares 9 bitmap methods (Bitset, WAH, EWAH, PLWAH, CONCISE, VALWAH, SBH, BBC, Roaring) with 12 list methods (VB, PforDelta, NewPforDelta, OptPforDelta, Simple9/16/8b, GroupVB, PEF, SIMDPforDelta, SIMDBP128 and starred variants).
- Data: synthetic uniform, zipf and markov distributions, plus 8 real datasets.
- [Wang et al. 2017 (PDF)](https://www.cs.purdue.edu/homes/csjgwang/pubs/SIGMOD17-Bitmap.pdf)

Lessons:
- **Space:** List codecs generally take less space than bitmaps "unless the list L is ultra dense." For uniform or markov lists, bitmaps (Roaring/Bitset) win only when the list is extremely long relative to the domain (a density threshold around |L|/d ≈ 1/5; the fraction is garbled in extraction). For zipf lists, SIMDPforDelta* takes the least space regardless of length.
- **Decompression:** List codecs are faster "in nearly all cases," and SIMDBP128* is best.
- **Intersection:** "Roaring bitmap achieves the fastest intersection performance among all the compression methods."
- **Union:** List codecs are generally better, and SIMDBP128* is "the fastest one in nearly all cases."
- **Recommendations:** Recommends SIMDBP128*, SIMDPforDelta* and Roaring overall. Advises "Use Roaring for bitmap compression whenever possible" and not BBC, WAH, EWAH, PLWAH, CONCISE, VALWAH or SBH.
- **Skip pointers:** "Be sure to build skip pointers on compressed inverted lists. It will not increase the space overhead that much (3% to 5%), but can improve the intersection performance dramatically."
- **Warning:** "A compression method that is good for decompression may not be good for intersection, and a compression method that is good for intersection may also not good for union."
- All points above: [Wang et al. 2017](https://www.cs.purdue.edu/homes/csjgwang/pubs/SIGMOD17-Bitmap.pdf)

**Lemire & Boytsov, "Decoding billions of integers per second through vectorization" (SPE 2015; arXiv 1209.2137)**
- SIMD-BP128 is "nearly twice as fast" as the previously fastest schemes (varint-G8IU and PFOR) and saves up to 2 bits/int.
- SIMD-FastPFOR compresses to within 10% of Simple-8b while decoding about 2x faster.
- [Lemire & Boytsov (arXiv)](https://arxiv.org/abs/1209.2137)

**Zhu et al., "LICO" (SIGMOD 2026, PACMMOD Vol. 4 No. 3, June 2026): the newest large benchmark found**
- Re-runs the Pibiri–Venturini codec suite (VByte, OptVByte, BIC, Delta, Rice, PEF, DINT, FastPFor, OptPFor, Simple16, QMX) plus learned codecs LA-vector and LeCo-var.
- Data: ClueWeb12-B13 (8.7B docids), CCNews (13.9B) and WITD.
- On space, "variable-length codecs … such as PEF and Rice, achieve the highest compression ratios."
- On random access, "PEF achieves the best average latency per query."
- [Zhu et al. 2026 (PDF)](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)

### Inferences
- **Universe-partitioned vs list codecs, from the survey's numbers:**
  - Space: Roaring costs about 2.1x PEF's bits/int on Gov2 (6.63 vs 3.11), 2.0x on ClueWeb09 and 1.7x on CCNews.
  - AND: Roaring is about 4–8x faster than the best list codec (Gov2: 0.59 vs 2.53 ms for QMX; CCNews: 2.45 vs 18.60).
  - OR: Roaring is about 14–18x faster (Gov2: 3.66 vs 52.99 ms).
- Among list codecs, AND time differs by only about 1.5x between QMX/VByte/PEF/Opt-VByte/DINT/Opt-PFor/Simple16. So on space-constrained deployments PEF loses little AND speed while being among the smallest.
- The studies agree on a three-way split:
  1. Roaring-like hybrid bitmaps for raw intersection throughput and dense lists.
  2. SIMD bit-packing (SIMD-BP128/FastPFor style) for decode- and union-heavy workloads.
  3. Elias-Fano variants when space matters and queries are skip-heavy.

### Gaps
- Exact per-codec NextGEQ (ns) numbers from the survey were not legible in the extracted text. The survey's full OR table for DINT, Opt-PFor, Simple16 and BIC also could not be read reliably. The original PDF or github.com/jermp/2i_bench would be needed.
- Trotman's "Compression, SIMD, and Postings Lists" (ADCS 2014) and the QMX paper were not retrieved directly. QMX numbers here come from the Pibiri–Venturini and Mallia benchmarks only.
- The exact meaning of Wang et al.'s starred variants (SIMDBP128*, SIMDPforDelta*) was not extracted.
- No independent benchmark from 2021–2026, other than LICO's own evaluation, comparing Roaring against PEF and list codecs on the same IR corpora was found.

## Elias-Fano family: plain EF, partitioned EF, clustered EF, and NextGEQ

### Takeaway
Elias-Fano supports constant-time access to a bucket via select on the high-bits bitvector, followed by a short scan. NextGEQ therefore runs without decompressing the list. Partitioned EF (PEF) adapts per-partition encoding (EF / bitvector / all-ones run), which makes it one of the most space-efficient codecs while staying competitive for skip-heavy (AND, block-max) queries. It is slower than SIMD bit-packing for full decoding and union.

### Cited Findings
- **How NextGEQ works on EF:**
  - In the IR context, the successor query is called NextGEQ.
  - The high bits locate the right bucket: select0 on the high-bits bitvector finds it in constant time.
  - A search then runs inside a range containing up to U/n integers.
  - [Pibiri & Venturini survey §3.4](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **Partitioned EF (Ottaviano & Venturini, SIGIR 2014):**
  - The sequence is split into variable-length partitions whose upper bounds and sizes are stored in a first-level EF sequence.
  - Each partition is encoded in one of three ways: EF, a characteristic bitvector, or nothing, when the partition covers the whole universe range.
  - A dynamic program chooses the partition at cost at most (1+ε) times optimal.
  - [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **Measured performance:**
  - Space: PEF is second only to BIC (3.11 / 4.99 / 5.45 bits/int on Gov2 / ClueWeb09 / CCNews).
  - Decode: 0.76 / 1.10 / 1.31 ns/int.
  - AND: 2.91 / 13.48 / 21.17 ms, close to QMX's 2.53 / 11.78 / 18.60.
  - OR: noticeably slower (69.77 vs 52.99 ms on Gov2 for QMX).
  - [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **PEF under dynamic pruning:** PEF nearly matches the fastest codecs under BMW and especially VBMW (skip/lookup-heavy), while being less efficient under MaxScore. — [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **PEF random access:** PEF had the best random-access latency of all codecs tested in the LICO SIGMOD 2026 benchmark, "benefiting from its dedicated indexing structure and fast block decoding." — [Zhu et al. 2026](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
- **Clustered EF:** Pibiri & Venturini appears in the survey's timeline of codecs. — [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)

### Inferences
- **Why EF is favored for intersection:** NextGEQ cost is about one select plus a short scan, independent of list length. This lets a short list drive skips through a long list without decoding the long list's blocks. Block codecs must instead decode a whole 128-integer block once they land in it.
- **When the advantage shrinks:** On OR queries or full-list scans, EF's bit-level decode (around 1 ns/int) loses to SIMD bit-packing, which explains PEF's weaker OR and MaxScore numbers.

### Gaps
- Vigna's original "Quasi-succinct indices" (WSDM 2013) numbers were not retrieved.
- Clustered EF (Pibiri & Venturini, TOIS 2017) space and time results were not retrieved; only its existence is confirmed via the survey's timeline.
- Standalone NextGEQ ns/op measurements per codec were not legible in the extraction.

## Block-based codecs (VByte family, PFor family, SIMD-BP128, Simple, QMX, BIC, DINT, ANS) and skip structures

### Takeaway
Block codecs (usually 128 integers per block) support set operations by decoding whole blocks, with skip pointers and block metadata to jump between blocks. The speed ranking is roughly:
- SIMD bit-packing ≈ SIMD VByte variants (fastest).
- PFor/Simple/QMX next.
- Entropy coders (BIC, ANS) far behind.

Compression runs in the reverse order. Dictionary coding (DINT) and Opt-VByte sit in the middle cluster. Skip pointers are essential, adding about 3–5% space for large intersection gains.

### Cited Findings
- **Block layout:** Block-wise encodings in PISA's benchmark used 128 postings per block. — [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **VByte family:** VarintGB, Varint-G8IU and StreamVByte are the least space-efficient: about 10–12 bits/docid on Gov2 regardless of ordering, and about 8.8–10 bits per freq. They gain little from reordering. — [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **Opt-VByte (Pibiri & Venturini):** It cuts VByte's space from 8.81 to 3.88 bits/int on Gov2, with decode around 0.7–0.9 ns/int and AND time close to VByte (2.98 vs 2.76 ms). — [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **SIMD-BP128 and SIMD-FastPFOR:** SIMD-BP128 is nearly 2x faster than varint-G8IU and PFOR and saves up to 2 bits/int. SIMD-FastPFOR is within 10% of Simple-8b's compression at about 2x Simple-8b's decode speed. — [Lemire & Boytsov](https://arxiv.org/abs/1209.2137)
- **SIMD-BP128 and VBMW:** SIMD-BP128 and the VByte variants are the fastest under VBMW and MaxScore. SIMD-BP128 roughly halves VByte's space on reordered indexes (5.42 vs 10.09 bits/docid on Gov2 BP). — [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **OptPFD and Simple:** OptPFD, Simple16 and Simple8b "fall slightly yet noticeably behind" SIMD-BP128 and VByte in query speed, with better compression. — [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **Middle cluster:** Opt-PFor (3.63 bits/int Gov2), Simple16 (4.18) and DINT (3.53) sit on similar tradeoffs. — [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **QMX:** QMX has the fastest AND among list codecs in the survey (2.53 ms Gov2) at 5.12 bits/int. — [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **Entropy coders (BIC, ANS):**
  - BIC is the most compact (2.94 bits/int Gov2) but the slowest, at 10.01 ms AND on Gov2, about 3.4x PEF. — [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)
  - Packed+ANS2 (Moffat & Petri) and Interpolative compete for the smallest index (Gov2 random: 7.36 GB and 7.32 GB) but are the slowest at query time (e.g., MaxScore 13.91 and 16.53 ms vs 6.27 ms for SIMD-BP128). — [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)
- **Skip pointers:** Building skip pointers on compressed lists costs about 3–5% space and "can improve the intersection performance dramatically." — [Wang et al. 2017](https://www.cs.purdue.edu/homes/csjgwang/pubs/SIGMOD17-Bitmap.pdf)
- **Group VByte vs PforDelta:** GroupVB is reported to be 1.6x faster in decompression than PforDelta in one synthetic setting, with VB taking 1.76x more space. (Fragmentary extraction; context is the zipf synthetic data.) — [Wang et al. 2017](https://www.cs.purdue.edu/homes/csjgwang/pubs/SIGMOD17-Bitmap.pdf)

### Inferences
- **Choosing a block codec:** For AND-heavy workloads, block codecs plus skip pointers are bottlenecked by "skip to block, then decode the whole block." The cheapest per-block decode wins (QMX, SIMD-BP128, VByte-SIMD), even if NextGEQ granularity is coarser than EF's. When index size matters, Opt-PFor, DINT and Opt-VByte offer about 30–60% less space than SIMD-BP128/VByte at modest speed cost.

### Gaps
- **DINT's venue:** DINT (Pibiri, Petri, Moffat) is cited by the task as SIGIR 2019. My recollection is that it appeared at WSDM 2019 as "Fast Dictionary-Based Compression for Inverted Indexes," but I did not retrieve the paper to confirm the venue or its standalone numbers.
- Standalone decode speeds for Masked VByte, Stream VByte and Varint-G8IU (from Lemire's papers) were not retrieved in this pass.
- Original ANS-for-IR paper numbers (Moffat & Petri, 2017/2018) were not retrieved; only the Packed+ANS2 numbers from Mallia et al.

## Bitmap-based representations (Roaring, WAH/EWAH/Concise, hybrids) and comparisons with list codecs

### Takeaway
Roaring dominates other compressed bitmaps:
- **vs RLE bitmaps:** several times, up to two orders of magnitude, faster than WAH, EWAH and Concise, while also smaller.
- **vs posting-list codecs:** the fastest intersection of any compressed representation in both Wang et al. 2017 and Pibiri & Venturini. It loses on space for sparse lists, often worse than VByte on web corpora.

Union and full decoding favor SIMD list codecs in Wang et al. In the IR survey, however, Roaring was also fastest for OR, by an order of magnitude.

### Cited Findings
- **Roaring layout:** Values are partitioned by their high 16 bits. A chunk holds a sorted 16-bit array when it contains ≤4096 values, else a 65,536-bit bitmap. — [Wang et al. 2017](https://www.cs.purdue.edu/homes/csjgwang/pubs/SIGMOD17-Bitmap.pdf); [Elastic blog (Grand, 2015)](https://www.elastic.co/cn/blog/frame-of-reference-and-roaring-bitmaps)
- **Roaring with run containers (Lemire, Ssi-Yan-Kai, Kaser, SPE 2016):**
  - The 2016 version adds RLE run segments to the uncompressed-bitmap and packed-array containers, because RLE bitmaps could be smaller on sorted data with long runs.
  - It is reported to be "several times faster (up to two orders of magnitude)" than WAH, Concise and EWAH while compressing better.
  - Production adopters listed: Lucene, Spark, Kylin, Druid.
  - [Lemire et al., arXiv 1603.06549](https://arxiv.org/abs/1603.06549)
- **Original Roaring paper:** It reported often around 2x better compression and up to 900x faster intersections versus WAH/Concise. — [Chambi et al., arXiv 1402.6407 (search snippet)](https://arxiv.org/pdf/1402.6407)
- **CRoaring (Lemire et al., SPE 2018):**
  - An optimized C library using SIMD-vectorized array intersection, union, difference and symmetric difference.
  - Systems listed as relying on Roaring: Elasticsearch, Spark, Druid, Pilosa, Hive.
  - [Lemire et al., arXiv 1709.07821](https://arxiv.org/abs/1709.07821)
- **Wang et al. on bitmaps vs lists:**
  - Roaring achieves the fastest intersection overall and is recommended for IR top-k candidate intersection, database conjunctive queries and star joins.
  - SIMDBP128* is recommended for disjunctive/range queries (union).
  - Bitset is not recommended because of space.
  - On TPC-H Q6, Roaring was faster than other methods "even faster than uncompressed list." On web data, Roaring "outperforms the other compression methods."
  - The paper contradicts the prior conclusion that inverted list compression always outperforms bitmap compression.
  - [Wang et al. 2017](https://www.cs.purdue.edu/homes/csjgwang/pubs/SIGMOD17-Bitmap.pdf)
- **Pibiri & Venturini on Roaring and Slicing:**
  - Roaring is the least space-effective, together with VByte (6.63 / 9.78 / 9.49 bits/int).
  - Roaring and Slicing are the fastest for decoding (0.49–0.70 ns/int), AND and OR.
  - Slicing (Pibiri, Venturini et al.'s universe-sliced hybrid) is smaller than Roaring (4.31 vs 6.63 bits/int Gov2) and about 1.5–1.75x slower.
  - [Pibiri & Venturini survey](https://ar5iv.labs.arxiv.org/html/1908.10598)
- **Lucene's 2015 filter-cache benchmark (Elastic):** It compared int[], a plain bitmap, Roaring and on-disk FOR postings.
  - **Iteration:** int[] was about 2x faster.
  - **Skipping:** Roaring "degraded most gracefully" as density changed; the plain bitmap was fastest on dense sets but slow on sparse ones.
  - **Memory:** Roaring used less than both, except for very sparse sets (<0.05%).
  - **Conclusion:** "roaring bitmaps are rarely the fastest implementation, they are never a bad choice."
  - [Elastic blog (Feb 17, 2015)](https://www.elastic.co/cn/blog/frame-of-reference-and-roaring-bitmaps)

### Inferences
- **Why Roaring wins AND despite poor compression:** Universe partitioning aligns containers across lists, so intersection becomes container-by-container work (bitmap AND, or SIMD array-array intersection). No per-element NextGEQ is needed. Bitmap-bitmap container ANDs are word-parallel.
- **Where Roaring loses:** Its 16-bit low-part arrays cannot exploit small d-gaps, which is the main source of compression after docid reordering. So it loses ground on web corpora with clustered docids (survey: worse than VByte on ClueWeb09/CCNews).
- **Union results disagree:** Wang et al. (union favors SIMDBP128*) and Pibiri & Venturini (Roaring fastest OR by about 14x) differ. The difference likely comes from data (synthetic/DB vs web posting lists) and implementation (Pibiri–Venturini use CRoaring's SIMD union vs heap-based multi-way merging for list codecs). This is a hypothesis, not established in the sources.

### Gaps
- Culpepper & Moffat's hybrid bitmap/list scheme ("Efficient set intersection for inverted indexing," TOIS 2010) was not retrieved; no numbers.
- No source found quantifying Concise/EWAH vs Roaring on standard IR corpora (Gov2/ClueWeb) beyond Wang et al.'s synthetic and real datasets.

## Newer approaches 2020–2026: learned compression, reordering, and other SOTA

### Takeaway
- **Learned compressors:** Learned compressors for sorted integers have matured from generic column codecs (LA-vector; LeCo, SIGMOD 2024) to inverted-index-specific, SIMD-aware designs. LICO (SIGMOD 2026) claims Pareto-optimal size/latency and native NextGEQ, intersection and union on the compressed form. It is still slightly larger than PEF/Rice and slower than PEF for random access.
- **Docid reordering:** BP/URL reordering remains one of the largest single levers on both compression (up to about 43% smaller) and query speed (up to about 3x).

### Cited Findings
- **LeCo (Liu, Zeng, Zhang, SIGMOD 2024):**
  - Fits learned models to runs of values and stores residuals; FOR, Delta and RLE are special cases.
  - Compared with FOR and Elias-Fano, it improves compression ratio by up to 91% with comparable decompression and random-access performance.
  - Its random access is an order of magnitude faster than Delta encoding, at competitive or better compression ratio.
  - End-to-end: up to 5.2x speedup in an Arrow analytical query and 16% higher RocksDB throughput.
  - [LeCo (arXiv 2306.15374, via search results)](https://arxiv.org/abs/2306.15374v3)
- **LICO (Zhu, Liu, Zhang, Sha, Liao, Hu, Chen; SIGMOD 2026, PACMMOD 4(3), Article 202, June 2026):** Code is at github.com/xianyuzhuruc/LICO. [Zhu et al. 2026](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
  - **Method:** Error-bounded piecewise-linear models plus residual arrays.
  - **Space model:** n sorted integers take 0.5·n·log2(σ²)+O(n) bits, where σ² is the gap variance. The authors call this "quasi-succinct compression comparable to Elias-Fano."
  - **Native NextGEQ:** The paper says LICO "natively supports NextGeq without any additional metadata."
  - **Intersection:** SIMD intersection passes on smaller lists, then NextGEQ for the remaining lists.
  - **Union:** SIMD bitonic-merge union.
  - **Space results:** PEF and Rice still achieve the highest compression ratios, followed by LICO++ and OptPFor. LICO++ improves on LICO's compression ratio by up to 1.29x, at about 0.3 ns/int extra decode.
  - **Speed results:** LICO has the highest decoding throughput. Its intersection beats "the fastest non-learned SIMD-based methods by up to 2.64×," and overall query performance improves "by up to 5.52×."
  - **Random access:** PEF remains best; LICO/LICO++ follow closely.
  - **Exclusions:** LA-vector and LeCo were excluded from the query-operation experiments because they "do not natively support list query operations." LICO notes LeCo "is not optimized for inverted index compression."
  - **Roaring not compared:** Roaring and Slicing are not in LICO's baseline list (VByte, OptVByte, BIC, Delta, Rice, PEF, DINT, FastPFor, OptPFor, Simple16, QMX).
- **Docid reordering (BP, recursive graph bisection, Dhulipala et al. KDD 2016):**
  - Packing methods are "highly sensitive to ordering."
  - URL ordering shrinks Packed+ANS2 by 43% (Gov2) and 27% (ClueWeb09), and "further improvements are seen for BP."
  - Random-order indexes can be "almost 3 times slower" to query than URL-ordered ones.
  - BP gives a consistent query-speed gain over URL on ClueWeb09, which the authors note had not been reported before.
  - The authors validated their BP implementation against Dhulipala et al.'s reported results.
  - [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

### Inferences
- **Learned compression's status:** As of late 2026, learned compression has its first inverted-index-specific peer-reviewed result (LICO) claiming to dominate classical list codecs on the space/decode Pareto curve. It has not been independently reproduced, and it does not beat PEF on space or random access. Comparisons against universe-partitioned Roaring/Slicing for AND/OR are absent.
- **Reordering interacts with codec choice:** It helps bit-packing, EF and entropy codecs far more than byte-aligned VByte. Systems that apply BP should therefore favor non-byte-aligned codecs.

### Gaps
- Mackenzie et al., "Compressing inverted indexes with recursive graph bisection: a reproducibility study" (ECIR 2019) was not retrieved; no direct numbers.
- No source was found on neural (non-PLA) lossless posting-list compression in 2021–2026.
- Other 2021–2026 SOTA (e.g., newer PISA codecs, BP variants, compressed-index work from Mackenzie/Petri/Moffat) was not surveyed in this pass.

## Which encodings are used in production systems?

### Takeaway
- **Lucene/Elasticsearch:** Bit-packed FOR blocks for docid deltas (no patching), patched FOR (PFOR) for frequencies, a VInt tail, and inlined two-level skip data with impacts. Roaring is used for the filter cache.
- **Tantivy:** A Rust port of simdcomp's 128-integer bit-packing.
- **Roaring elsewhere:** Widespread in analytic engines (Spark, Druid, Hive, Pilosa, Kylin).
- **PISA:** The research reference implementation of the full codec zoo.

### Cited Findings
- **Lucene 10.1 postings format (Lucene101PostingsFormat javadoc):**
  - Doc-delta and freq blocks are packed at a single bit width per block, with block size "fixed (currently 128)."
  - "PackedDocDeltaBlock does not use patching" while "PackedFreqBlock" "uses patching" (i.e., PFOR for freqs).
  - Blocks that are all the same value get an optimized encoding.
  - Leftover postings go in a VInt block, where doc and freq are combined.
  - "Level 0 skip data is interleaved between every packed block" and "Level 1 skip data is interleaved between every 32 packed blocks."
  - Impacts (competitive freq/norm pairs) are stored for score-based skipping.
  - ForUtil and ForDeltaUtil "encode multiple integers in one" Java int/long "to get SIMD-like speedups."
  - [Lucene 10.1 javadoc](https://lucene.apache.org/core/10_1_0/core/org/apache/lucene/codecs/lucene101/Lucene101PostingsFormat.html); [Lucene 10.2 package summary](https://lucene.apache.org/core/10_2_0/core/org/apache/lucene/codecs/lucene101/package-summary.html)
  - Block size conflict: the 2015 Elastic blog says Lucene splits postings into blocks of 256 docids, which disagrees with the javadoc's 128. — [Elastic blog](https://www.elastic.co/cn/blog/frame-of-reference-and-roaring-bitmaps)
- **Lucene history:** FOR-style delta+bit-packing has been in Lucene since 4.1. Lucene 5 replaced plain bitmaps in its filter cache with Roaring bitmaps. — [Elastic blog (Grand, Feb 2015)](https://www.elastic.co/cn/blog/frame-of-reference-and-roaring-bitmaps)
- **Tantivy:**
  - Uses SIMD bit-packing on blocks of 128 integers ("SIMDPack128"-style). It is a pure-Rust reimplementation of Lemire's simdcomp with delta encoding.
  - Each block has a full-byte bit-width header (its author notes 5–6 bits would suffice).
  - The tantivy_bitpacker crate's BlockedBitpacker uses 128-element blocks with an index.
  - [tantivy_bitpacker docs](https://docs.rs/tantivy-bitpacker); [fulmicoton blog](https://fulmicoton.com/posts/bitpacking/)
- **Roaring adopters:**
  - Lucene, Spark, Kylin, Druid. — [Lemire et al. 2016](https://arxiv.org/abs/1603.06549)
  - Elasticsearch, Spark, Druid, Pilosa, Hive. — [Lemire et al. 2018](https://arxiv.org/abs/1709.07821)
- **PISA:** PISA (github.com/pisa-engine/pisa), a fork of ds2i, implements VarintGB, VarintG8IU, OptPFD, Simple16/8b, SIMD-BP128, StreamVByte (via FastPFor), PEF, Interpolative, QMX and ANS. — [Mallia et al. 2019](https://research.engineering.nyu.edu/~suel/papers/daat-ecir19.pdf)

### Inferences
- **Production choices vs research:** Production engines (Lucene, Tantivy) chose the SIMD bit-packing / FOR family plus multi-level skip lists and per-block impacts, matching the research finding that SIMD-BP128-like codecs give near-VByte speed at much better compression. None of the mainstream engines found here uses EF/PEF or entropy coders for postings. Roaring is used where set algebra over cached or filter sets dominates.

### Gaps
- **Lucene 10.x block size and bitset encoding:** I could not confirm from CHANGES.txt whether Lucene 10.x changed the postings block size (128 vs 256) or added bitset encoding for dense docid blocks. The retrieved javadoc says 128, which conflicts with the Elastic blog's 256.
- **Systems not verified:** Vespa's posting-list encoding was not researched. ClickHouse and Pinot use of Roaring was not verified with a primary source in this pass; only Druid/Spark/Hive/Pilosa/Kylin/Elasticsearch/Lucene are sourced.
- **Tantivy skip and block-max layout:** The on-disk layout of Tantivy's skip index and block-max metadata was not confirmed.
