# Algorithms for Intersection and Union of Sorted Integer Lists (Posting Lists): Classic Results to State of the Art (as of Oct 2026)

Notation used below: k = number of lists; n_1 ≤ … ≤ n_k list sizes (m = small list, n = large list in the two-list case); N or n = total input size; r = output size; u = universe size; w = machine word size; δ = Barbay–Kenyon alternation (size of the smallest partition certificate); G/D = Demaine et al. gap cost / instance difficulty.

## 1. Classic pairwise intersection algorithms and their cost bounds

### Takeaway
For two sorted lists of sizes m ≤ n, the comparison lower bound is about log2 C(n+m, m) ≈ m·log(n/m) comparisons (Hwang–Lin, 1972). Linear merge costs O(m+n). m independent binary searches cost O(m log n). Galloping (doubling or exponential) search and Baeza-Yates' divide-and-conquer both reach O(m log(n/m)), so they adapt smoothly between the two extremes. In practice, merge wins until the size ratio reaches roughly 20–50×. Past that point, galloping and binary-search methods win.

### Cited Findings
- **Linear merge.** Scanning both lists in parallel (the merge step of merge sort) costs O(|L1|+|L2|) operations. Ding & König (2011) call it "wasteful" when sizes differ a lot or little of the lists intersect. — [Ding & König, PVLDB 4(4) 2011](https://arxiv.org/pdf/1103.2409)
- **Hwang–Lin bound (1972).** For |L1| < |L2|, algorithms that exploit size asymmetry need at most about log C(|L1|+|L2|, |L1|) + |L1| comparisons. Ding & König cite this to Hwang & Lin, "A Simple Algorithm for Merging Two Disjoint Linearly Ordered Sets," SIAM J. Computing 1(1):31–39, 1972. — [Ding & König 2011](https://arxiv.org/pdf/1103.2409)
- **Hwang–Lin average case.** Fernandez de la Vega et al. analyzed the average case of a simplified Hwang–Lin binary merge and found it to be O(m log(n/m)). They also gave a probabilistic algorithm that beats Hwang–Lin in the worst case for 1.618m ≤ n ≤ 3m. For ordered sets, lower bounds for intersection are also lower bounds for merging, but not the other way round. — [Baeza-Yates & Salinger, "Fast Intersection Algorithms for Sorted Sequences," LNCS 6060, 2010](https://cs.uwaterloo.ca/~ajsaling/papers/intersection_alg_app10.pdf)
- **Binary search.** When m is small (m = o(n/lg n)), m binary searches cost O(m lg n), which is better than merging. Each binary search can be restricted to the part of the large list not yet processed. — [Baeza-Yates & Salinger 2010](https://cs.uwaterloo.ca/~ajsaling/papers/intersection_alg_app10.pdf)
- **Galloping, doubling or unbounded search (Bentley–Yao 1976).** Searching for d sorted elements in an array of size n_i costs d·log n_i comparisons with binary search. Unbounded (doubling) search costs fewer than 2d·log(n_i/d), because each search costs at most 2 log(gap) and the log is concave. — [Barbay & Kenyon, ACM TALG 4(1), 2008](https://users.dcc.uchile.cl/~jbarbay/Publications/2008-TALG-AlternationAndRedundancyAnalysisOfTheIntersectionProblem-BarbayKenyon.pdf)
- **Galloping intersection cost.** Galloping intersection runs in O(min(n1,n2)·log max(n1,n2)) time. Lemire et al. note that a merge-like O(m + n/T) SIMD scan is beaten by galloping when n ≫ m. — [Lemire, Kaser et al., Roaring library paper, SPE 2018](https://arxiv.org/html/1709.07821v6); [Lemire, Boytsov, Kurz, "SIMD Compression and the Intersection of Sorted Integers," arXiv 1401.6399 (SPE 2016)](https://arxiv.org/pdf/1401.6399)
- **Baeza-Yates' algorithm (CPM 2004; SPIRE 2005 experiments; 2010 journal-style version).** The algorithm takes the median of the smaller list, binary-searches for it in the larger list, and recurses on both sides. This is a "balanced version of Hwang and Lin's algorithm."
  - Worst case: W(m,n) = 2(m+1)·lg((n+1)/(m+1)) + 2m + O(lg n). That is O(m lg n) for small m and O(n) when n = αm.
  - Average case: A(m,n) = (m+1)(ln((n+1)/(m+1)) + 3 − 1/ln 2) + O(lg n). It uses fewer comparisons than m+n on average and improves on merging once α = n/m ≥ 2.
  - — [Baeza-Yates & Salinger 2010](https://cs.uwaterloo.ca/~ajsaling/papers/intersection_alg_app10.pdf)
- **Measured merge crossover.** Baeza-Yates & Salinger fit the CPU-time crossover where Merge becomes faster than their recursive Intersect: merge wins when m > 0.033n + 8.884 (r² = 0.999), i.e., size ratios below about 30:1. A refined hybrid (Hybrid2) switches to Merge when m > 0.028n + 32.5. They stress that the cut point depends on data, implementation and hardware. — [Baeza-Yates & Salinger 2010](https://cs.uwaterloo.ca/~ajsaling/papers/intersection_alg_app10.pdf)
- **Modern SIMD-era crossover (context only).** Lemire et al. (2016) found their merge-like vectorized V1 fastest up to about 16:1 size ratio and SIMD galloping fastest above about 1024:1. Their production heuristic uses V1 below 50:1, V3 from 50:1 to 1000:1, and SIMD galloping above that. SIMD intersection was "up to twice as fast" as scalar galloping. — [Lemire, Boytsov, Kurz 2016](https://arxiv.org/pdf/1401.6399)
- **Hashing.** A hash table on the larger list gives expected O(min(|L1|,|L2|)) time. That is a factor Θ(log(1 + n/m)) better than the best possible comparison-based worst case. However, the indirection makes it perform poorly when sizes are less skewed. — [Ding & König 2011](https://arxiv.org/pdf/1103.2409)
- **Trees, treaps and skip lists.** These give O(|L1|·log(|L2|/|L1|)) intersection of preprocessed sets. They are "typically not used" for static IR data because of the space overhead. — [Ding & König 2011](https://arxiv.org/pdf/1103.2409)

### Inferences
- All asymmetric methods (binary search, galloping, Baeza-Yates, Hwang–Lin) match the information-theoretic Θ(m log(n/m)) bound within constant factors. The practical differences come from constants, branch prediction, cache behavior and, for compressed lists, the cost of random access (see Q3 and Q6).
- Merge-vs-gallop crossovers vary by era and implementation: about 30:1 (Baeza-Yates & Salinger, scalar, 2005–2010), about 50:1 (Lemire et al., SIMD, 2016) and 64:1 (CRoaring, see Q6). The consensus is to merge similar-size lists and gallop or skip when sizes are skewed by more than about 30–64×.

### Gaps
- I did not read Hwang & Lin (1972) or Bentley & Yao (1976) directly. Their bounds are cited through secondary academic sources (Ding & König; Barbay & Kenyon; Baeza-Yates & Salinger).

---

## 2. Adaptive and multi-way intersection algorithms, and what experiments say is best in practice

### Takeaway
Theory has a hierarchy of adaptive measures and algorithms:
- Demaine–López-Ortiz–Munro (SODA 2000): gap-cost/proof-based measure, round-robin "Adaptive" algorithm, optimal up to a factor of k.
- Barbay–Kenyon (SODA 2002; TALG 2008): alternation δ, "Sequential" algorithm, an optimal Θ(δ Σ log(n_i/δ)) bound, plus randomized variants that are strictly better in redundancy analysis.

Large experimental studies consistently find that the simple **SvS (set-vs-set, smallest-first) with galloping search** is the best or near-best in CPU time on real query logs:
- Barbay, López-Ortiz, Lu, Salinger (JEA 2009)
- Culpepper & Moffat (TOIS 2010)

Small_Adaptive minimizes comparisons. Baeza-Yates wins only on uniform random data. Holistic round-robin algorithms (Sequential, Adaptive) lose in practice.

### Cited Findings
**Demaine, López-Ortiz, Munro, "Adaptive Set Intersections, Unions, and Differences" (SODA 2000, pp. 743–752)**
- They define a "proof" (certificate) for each instance and give adaptive algorithms "within a constant factor of optimal with respect to a natural measure of the difficulty of an instance." The paper also sets out a general framework for adaptive analysis in the comparison model. — [Demaine et al., SODA 2000 page](https://erikdemaine.org/papers/SODA2000/)
- Intersection proof cost = set cost s(P) = |P|·(log2 k + log2(k−1)) plus gap cost g(P). The gap cost is the sum, over sets, of the log-gaps between compared elements, excluding the largest gap in each set.
- Theorem 5.1: algorithm Empty-Intersect runs in O(kG) time and makes at most 8kG comparisons.
- Theorem 4.1: there are instances with difficulty D = O(p log² k + g) where any algorithm takes Ω(kG). So a factor-k gap relative to the information-theoretic difficulty is unavoidable in their scaled-running-time framework.
- — [Demaine et al., SODA 2000 full paper (PostScript)](https://erikdemaine.org/papers/SODA2000/paper.ps)
- **ALENEX 2001 follow-up** ("Experiments on Adaptive Set Intersections for Text Retrieval Systems"). They compared Adaptive against a straightforward algorithm on a Google web-text index, measuring comparisons, and proposed a better hybrid (Small_Adaptive). The abstract describes an adaptive k-set intersection algorithm within a factor of at most 8k comparisons of the information-theoretic lower bound. — [Demaine et al. ALENEX 2001](https://erikdemaine.org/papers/ALENEX2001); [Barbay et al. JEA 2009](https://cs.uwaterloo.ca/~alopez-o/files/AEIoSIAfTS_2009.pdf)

**Barbay & Kenyon, alternation analysis (SODA 2002, pp. 390–399; journal version ACM TALG 4(1), Art. 4, March 2008)**
- Alternation δ is the size of the smallest partition certificate, i.e., the nondeterministic complexity of the instance.
- Theorem 3.3 (lower bound): for any k ≥ 2, 0 < n_1 ≤ … ≤ n_k and δ ∈ {4,…,4n_1}, any randomized algorithm performs Ω(δ Σ_{i=1..k} log(n_i/δ)) comparisons on average on some instance of alternation ≤ δ.
- Theorem 3.4: a deterministic algorithm achieves O(δ Σ log(n_i/δ)), so the bound is tight and deterministic algorithms are optimal under alternation.
- Redundancy analysis (Barbay 2003): the randomized algorithm Rand Intersection achieves O(ρ Σ log(n_i/ρ)) on average, and no deterministic algorithm is optimal under redundancy. Randomization therefore strictly helps in the comparison model.
- — [Barbay & Kenyon, TALG 2008](https://users.dcc.uchile.cl/~jbarbay/Publications/2008-TALG-AlternationAndRedundancyAnalysisOfTheIntersectionProblem-BarbayKenyon.pdf)
- The SODA 2002 lower bound implies the Demaine–López-Ortiz–Munro algorithm is "usually optimal" in the adaptive sense. — [Barbay & Kenyon SODA 2002 (academia.edu copy)](https://www.academia.edu/778359/Adaptive_intersection_and_t_threshold_problems)
- Later summary of both bounds: Barbay–Kenyon runs in optimal O(δ Σ lg(n_i/δ)). Demaine et al. run in O(kδ lg(n/δ)), which is optimal when max_i lg n_i = O(min_i lg n_i). — [Arroyuelo & Castillo, arXiv 2212.00946 (2022)](https://arxiv.org/pdf/2212.00946)

**Algorithm definitions (from Barbay, López-Ortiz, Lu, Salinger, JEA 14, Art. 3.7, 2009)**
- **SvS:** sort sets by size and intersect two at a time starting from the two smallest. Each candidate is searched in the next set from the last found position.
- **Swapping_SvS** (originally Hwang et al.): picks the searched element from the set with fewest remaining elements.
- **Small_Adaptive:** for each element of the smallest set, gallops in the second-smallest set; on a match, it checks the remaining k−2 sets. It re-sorts sets by remaining size at each step and outputs results in order.
- **Sequential** (Barbay–Kenyon): cycles through sets doing one full gallop per set, so it performs at most k searches per comparison of an optimal nondeterministic algorithm.
- **RSequential:** picks the next set at random.
- **BaezaYates / So_BaezaYates:** So_BaezaYates is the authors' variant that avoids re-sorting intermediate results.
- All of these are linear in the worst case, and each wins on some instances.
- — [Barbay et al. JEA 2009](https://cs.uwaterloo.ca/~alopez-o/files/AEIoSIAfTS_2009.pdf)

**Experimental conclusions (Barbay et al., JEA 2009)**
- Data: random sets, Google queries (average 2.286 keywords per query), and TREC GOV2 (361 GB) with the Terabyte 2006 efficiency queries. Machines: one CISC and one RISC.
- Fewest searches: Small_Adaptive, SvS and Swapping_SvS on random data; Small_Adaptive on real data.
- Fewest comparisons: Small_Adaptive with the new value-based Extrapol_Ahead search on real data.
- CPU time:
  - On random data, BaezaYates with Adaptive_Binary is best, "closely followed by SvS using Galloping."
  - On real data, "SvS takes the advantage … in combination with Galloping search."
- Sequential and RSequential perform poorly, because "most instances have a long certificate."
- Final recommendation: the best solution "seems to be one of the simplest melding algorithm SvS, already used in practice, … improved by replacing … Adaptive_Binary … by … Galloping search."
- Interpolation search uses fewer comparisons but costs more CPU, especially on RISC (SUN).
- — [Barbay et al. JEA 2009](https://cs.uwaterloo.ca/~alopez-o/files/AEIoSIAfTS_2009.pdf)
- Secondary summary with numbers: on uniform synthetic data, Baeza-Yates was about 30% faster than SvS+galloping. On real data (TREC GOV2), SvS+galloping was "superior to most alternatives by a wide margin (e.g., 2× faster)." — [Lemire, Boytsov, Kurz 2016](https://arxiv.org/pdf/1401.6399)

**Ding & König (2011) on adaptive methods**
- Adaptive algorithms minimize comparisons, but "the resulting reduction in the number of comparisons does not necessarily result in performance improvements in practice."
- In their benchmarks, the adaptive family (SvS, Adaptive, BaezaYates, SmallAdaptive) was generally slower than Merge, Lookup and their own methods. The exception was real Bing/Wikipedia queries, where SvS beat Merge and Lookup.
- — [Ding & König 2011](https://arxiv.org/pdf/1103.2409)

**Ordering lists for k-way pairwise (left-deep) intersection**
- A patent on greedy ordering of AND-trees states that the greedy method stays within a factor O(k) of the minimum certificate size. It also states that some input families force any deterministic or randomized method to use k times the comparisons in the certificate. — [USPTO patent 7925604](https://image-ppubs.uspto.gov/dirsearch-public/print/downloadPdf/7925604)

### Inferences
- The theory–practice gap is well documented. Holistic, round-robin "certificate-chasing" algorithms (Adaptive, Sequential) are asymptotically superior on instances with short certificates. Real query logs, however, mostly have long certificates and skewed list sizes. That favors SvS (smallest-first) with galloping, which exploits size skew and sequential memory access.
- The "leapfrog" k-way scheme used in Lucene, PISA and Leapfrog Triejoin (Q5–Q6) is essentially a round-robin or lead-driven variant of the Demaine/Barbay–Kenyon family. It probes with `advance()`/NextGEQ (galloping or skip-based), with the rarest list as the lead.

### Gaps
- I could not access the ALENEX 2001 full text to get its exact comparison counts.
- I did not read Tsirogiannis, Guha & Koudas, "Improving the Performance of List Intersection" (PVLDB 2009). It surfaced in search but its findings were not verified.

---

## 3. Culpepper & Moffat (TOIS 2010), Sanders & Transier (ALENEX 2007), Ding & König (VLDB 2011), Kane & Tompa, and other theoretical results

### Takeaway
Three engineering results stand out:
- **Culpepper & Moffat:** store the densest lists as bitvectors and the rest as compressed, byte-coded lists intersected with SvS+galloping. This beats even uncompressed arrays.
- **Sanders & Transier:** a "Lookup" two-level bucket index on compressed lists gives intersection in expected time linear in the smaller list.
- **Ding & König:** randomized hash partitions and word-level bitmaps give expected O(n/√w + kr) and were 5–15× faster than compressed Merge or Lookup, at a larger space cost.

On the theory side, Arroyuelo & Castillo (2022, journal version apparently 2026) show that compressed binary tries achieve adaptive O(kδ lg(u/δ)) intersection in compressed space. In experiments they beat PEF and Roaring by about 1.1–3.3×. The "Kane–Kowalski" result named in the brief could not be found. The likely intended reference is **Kane & Tompa** (SIGIR 2014, skewed partial bitvectors).

### Cited Findings
**Culpepper & Moffat, "Efficient Set Intersection for Inverted Indexing," ACM TOIS 29(1):1.1–1.25, Dec 2010 (DOI 10.1145/1877766.1877767)**
- Studies the trade-off between compressed and uncompressed storage when intersection uses random-access techniques. It proposes a "simple hybrid" that gives compact storage and conjunctive queries faster than uncompressed representations. — [Moffat abstract page](https://people.eng.unimelb.edu.au/ammoffat/abstracts/cm10acmtois.html)
- **HYB+M2 scheme (as summarized by Lemire et al.):**
  - Lists whose average gap is ≤ B (B = 8, 16 or 32) are stored as bitmaps. All other lists are stored as compressed deltas (variable-byte in Culpepper & Moffat).
  - Short lists are first intersected SvS-style, two by two from the smallest, and the resulting candidates are then checked against the bitmaps.
  - Culpepper & Moffat found SvS with galloping fastest; their own "max" algorithm was also fast.
  - An interpolative-like search was slightly better (<10%) for queries with ≥ 9 terms.
  - Skipping was counterproductive when bitmaps are used.
  - — [Lemire, Boytsov, Kurz 2016](https://arxiv.org/pdf/1401.6399)
- **Precursors:**
  - ADCS 2007 ("Hybrid Bitvector Index Compression"): storing "a small fraction of the inverted lists" as bitvectors has "no or negligible effect on compressed index size" versus byte codes, while query processing is faster than both byte-coded and all-bitvector indexes. — [Moffat & Culpepper ADCS 2007](https://people.eng.unimelb.edu.au/ammoffat/abstracts/mc07adcs.html)
  - SPIRE 2007 ("Compact Set Representation for Information Retrieval," LNCS 4726, pp. 137–148): a simple, low-cost auxiliary index means compressed storage "need not hinder efficient and high-speed intersection operations" relative to the best array-based algorithms. — [Culpepper & Moffat SPIRE 2007](https://people.eng.unimelb.edu.au/ammoffat/abstracts/cm07spire.html)
- **SIMD follow-up (2016).** Lemire, Boytsov & Kurz kept the HYB+M2 framework and added SIMD decoding and SIMD intersection, which "nearly double[d] the speed" of HYB+M2 on realistic data and queries. — [Lemire et al. 2016](https://arxiv.org/pdf/1401.6399)

**Kane & Tompa (SIGIR 2014, pp. 263–272, DOI 10.1145/2600428.2609609); Kane PhD thesis (Waterloo 2014)**
- They improved HYB+M2 with auxiliary structures to skip over large blocks (256 integers) of compressed values. This contrasts with Culpepper & Moffat's finding that skipping is counterproductive alongside bitmaps. — [Lemire et al. 2016](https://arxiv.org/pdf/1401.6399); [SIGIR 2014 anthology entry](https://ir.webis.de/anthology/2014.sigirconf_conference-2014.28/)
- The thesis combines bitvectors for large lists with large skips overlaid on compressed lists, and introduces "semi-bitvectors": the front of a list, a dense region created by document-size grouping and URL ordering, is stored as a bitvector, and the rest as skips over compressed values. It also shows that partitioning the index by document size creates dense regions that bitvectors can exploit. — [Kane thesis 2014](https://uwspace.uwaterloo.ca/bitstream/10012/8945/3/Kane_Andrew.pdf)

**Sanders & Transier, "Intersection in Integer Inverted Indices," ALENEX 2007, pp. 71–83; Transier & Sanders, ALENEX 2008 and ACM TOIS 2010**
- Bibliographic confirmation. — [KIT Algorithm Engineering](https://ae.iti.kit.edu/english/737.php)
- Two-level structure: document IDs are bucketed by their most significant bits, and a lookup table gives direct access to each bucket's first value. KIT lecture slides tie this to the "Lookup" intersection algorithm. — [KIT lecture slides](https://ae.iti.kit.edu/download/ti_lec9_1.pdf)
- Transier & Sanders' TOIS 2010 article, "Engineering basic algorithms of an in-memory text search engine," presents Lookup, a structure allowing intersection in expected time linear in the smaller list. — [Crossref record, TOIS 2010](https://api.crossref.org/works/10.1145%2F1877766.1877768)
- Ding & König benchmarked Lookup with bucket size B = 32, "the best value in our and the authors' experience." It was usually the best non-new method after Merge. — [Ding & König 2011](https://arxiv.org/pdf/1103.2409)

**Ding & König, "Fast Set Intersection in Memory," PVLDB 4(4):255–266, 2011 (Microsoft Research)**
- Theory:
  - Linear-space structures give expected O(n/√w + kr) time for k preprocessed sets (IntGroup/RanGroup).
  - A simpler RanGroupScan runs in O(n/α^m + mn/√w + kr√w).
  - HashBin handles skewed pairs in O(n1 log(n2/n1)).
  - The previous best asymptotic bound was Bille–Pagh–Pagh (ISAAC 2007), O((n log² w)/w + kr), described as "slow in practice" because of complex bit manipulation.
- Motivation: among the 10K most frequent Bing Shopping queries, for 94% the full intersection was at least an order of magnitude smaller than the rarest term's list, and for 76% two orders of magnitude smaller.
- Synthetic results:
  - RanGroupScan was 40–50% faster than Merge on equal-size lists.
  - Merge becomes fastest when the intersection is large (r > 7M out of 10M elements).
  - Hash and SkipList performed poorly except at extreme skew.
- Real data (Bing queries over 8M Wikipedia documents): RanGroupScan was best for 61.6% of queries, RanGroup 16%, HashBin 7.7%, Lookup 6.4%, SvS 3.6%.
- Space without compression: 37–87% more than a plain posting list.
- Compressed comparison:
  - RanGroupScan_Lowbits was 7.6–15× faster than Merge and 7.4–13× faster than Lookup (synthetic).
  - On real data it was 8.4× faster than Merge+δ-coding and 5.7× faster than Lookup+δ.
  - Its worst-case query latency was 5.2× lower than Merge+δ.
  - The cost was space: 66% of uncompressed, versus 26–28% for Merge and 35–37% for Lookup.
- — [Ding & König 2011](https://arxiv.org/pdf/1103.2409)
- Lemire et al. summarize Ding & König's RanGroup as about 10–30% better than SvS+galloping. It works on a specialized randomly partitioned structure rather than sorted lists, and Merge was faster in some cases. — [Lemire et al. 2016](https://arxiv.org/pdf/1401.6399)

**Arroyuelo & Castillo, "Trie-Compressed Intersectable Sets" (arXiv 2212.00946, Dec 2022)**
- Represent each set as a compact binary trie, following Trabb-Pardo's classic trie intersection, and introduce "trie certificates."
- Show that Trabb-Pardo intersection is adaptive: O(kδ lg(u/δ)) time, in space bounded by trie entropy.
- Experiments (Gov2, ClueWeb09, CC-News):
  - 1.55–3.32× faster than Partitioned Elias-Fano (PEF) using 1.15–1.72× its space.
  - 1.07–1.91× faster than Roaring using 0.48–1.01× its space.
  - 1.04–2.18× faster than RUP using 0.82–1.19× its space.
- — [arXiv 2212.00946](https://arxiv.org/pdf/2212.00946)
- An ACM-published version titled "Adaptive Set Intersection via Trie Partition Certificates" states that it revisits Trabb-Pardo's algorithm in the pointer-machine model and proves an adaptive bound matching the optimal comparison-model bound of Barbay & Kenyon. — [ACM DL 10.1145/3812648](https://dl.acm.org/doi/10.1145/3812648) (the page returned HTTP 403; claim taken from the search-result snippet)

**Fine-grained and competitive lower bounds (theory, 2014–2025)**
- Kopelowitz, Pettie, Porat (arXiv 2014; SODA 2016) give efficient reductions from 3SUM to offline SetDisjointness and SetIntersection (reporting), with polynomially higher conditional lower bounds than Pătraşcu (2010). — [arXiv 1407.6756](https://arxiv.org/abs/1407.6756)
- For online (preprocessed) set intersection they give a 3SUM-based tradeoff p + 2q ≥ 2 (p = preprocessing exponent, q = query exponent), tight for 0 ≤ q ≤ 2/3. No lower bound was known for 2/3 ≤ q < 1 as of 2020. — [Kopelowitz & Vassilevska Williams, ICALP 2020](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ICALP.2020.74)
- van der Hoog, Rotenberg, Rutschmann, "Tight Better-Than-Worst-Case Bounds for Element Distinctness and Set Intersection" (arXiv, 4 Nov 2025). For deterministic comparison-based set intersection, no algorithm can be o(log n)-competitive with the best algorithm on each "structure" class, and they give an O(log n)-competitive algorithm. The ratio is therefore Θ(log n), versus Θ(log log n) for element distinctness. — [arXiv 2511.02954](https://arxiv.org/abs/2511.02954)

### Inferences
- The consensus from Culpepper–Moffat, Kane–Tompa and Lemire et al.: with compressed lists, use **hybrid representations** (bitvectors for dense lists, block-compressed lists with skips for sparse ones). Drive the intersection from the rarest list using SvS+galloping or NextGEQ, and finish with bitmap probes. That pattern recurs in Roaring's container design and Lucene's BitSetConjunctionDISI (Q6).
- Ding & König's large speedups come from extra space and non-standard layouts (hash partitions). Production engines have mostly not adopted them, preferring sorted, block-compressed lists with skip data.
- The 2025 Θ(log n) competitive-ratio result suggests per-instance ("universal") optimality for set intersection is fundamentally harder than for element distinctness. It does not change practical algorithm choice.

### Gaps
- **"Kane–Kowalski":** no paper by authors "Kane & Kowalski" on set intersection was found in searches. The closest match is Kane & Tompa (SIGIR 2014), which Lemire et al. cite as improving Culpepper & Moffat. Treat "Kane–Kowalski" as possibly a misattribution.
- I could not access the full Culpepper & Moffat TOIS 2010 text. Exact speedup numbers and the precise bitvector-selection rule are taken from Lemire et al.'s secondary description (lists with average gap ≤ B, B ∈ {8, 16, 32}).
- I could not access the Sanders & Transier ALENEX 2007 full text. Its exact measured speedups are unverified.
- The venue and year of the ACM version of Arroyuelo & Castillo (DOI 10.1145/3812648) are unconfirmed (page blocked). The DOI pattern suggests 2025–2026.

---

## 4. Union, t-threshold and difference: algorithms and bounds

### Takeaway
- **Union** output size is Ω(n) in the worst case, so practical union is merge-based: heap-based k-way merge, or windowed bitset accumulation (Lucene's 2048-doc windows, Roaring's bitmap ORs). Its speed is governed by sequential decoding speed, not skipping.
- Adaptively, Demaine et al. (SODA 2000) give an O(D)-time union proof algorithm that is optimal in their difficulty measure, plus O(kG)-time union and difference as persistent B-trees.
- Barbay & Kenyon extend alternation analysis to **t-threshold** (elements in ≥ t of k sets). No lower bound is known for t < k/2.
- **Difference** is equivalent to intersection in Demaine et al.'s proof framework.

### Cited Findings
- **Demaine et al. (SODA 2000), problems covered.** They consider intersection, union and difference of k sorted sets.
  - Intersection and difference proofs are "identical" in structure (Lemma 2.3).
  - The cost of a union instance is a set cost s(I) = Σ_{i=1..#(I)} log2 C(k, #I(i)), where #I(i) is the number of sets containing the i-th distinct value, plus a gap cost. They argue this encoding is information-theoretically optimal (Theorem 3.2).
  - Theorem 5.2: algorithm Union-Proof runs in O(D) time, where D is the instance difficulty.
  - Theorem 6.1: the difference of k sorted sets stored as read-only B-trees can be computed as a B-tree in O(kG) time.
  - Theorem 6.2: the union of k sets stored as read-only B-trees can be computed as a B-tree in O(kG) time. The construction carves trees by the proof's partition, then merges pieces persistently using a priority queue keyed by tree height.
  - The adaptive union algorithm had been known for two sets; their contribution is the k-set generalization with matching lower bounds.
  - — [Demaine et al., SODA 2000 (paper.ps)](https://erikdemaine.org/papers/SODA2000/paper.ps)
- **Why output representation matters.** Demaine et al. note that if all elements of one set fall between a pair of elements of another, the union of two B-trees can be built from O(log n) new nodes referencing portions of the inputs. Union need not cost Θ(output) if the output can share structure. — [Demaine et al. 2000](https://erikdemaine.org/papers/SODA2000/paper.ps)
- **t-threshold (Barbay & Kenyon, SODA 2002).** They extend the lower bound and the algorithm to the t-Threshold Problem: find elements in at least t of the k sets. A later ISAAC 2003 paper gives a deterministic t-threshold algorithm and states that no lower bound is known for t < k/2. — [Barbay & Kenyon SODA 2002](https://www.academia.edu/778359/Adaptive_intersection_and_t_threshold_problems); [Deterministic algorithm for t-threshold (ISAAC 2003)](https://www.academia.edu/48716831/Deterministic_Algorithm_for_the_t_Threshold_Set_Problem)
- **Union-intersection expressions.** Bille, Pagh & Pagh, "Fast Evaluation of Union-Intersection Expressions" (ISAAC 2007), gave the then-best word-RAM bound O((n log² w)/w + kr), described by Ding & König as needing "complex bit-manipulation, making it slow in practice." — [Ding & König 2011](https://arxiv.org/pdf/1103.2409)
- **Two-set union via multiple search.** Good multiple-search (intersection) algorithms can also compute a two-set union with the same time complexity. — [Baeza-Yates & Salinger 2010](https://cs.uwaterloo.ca/~ajsaling/papers/intersection_alg_app10.pdf)
- **Empirical behavior on compressed indexes (Pibiri & Venturini, ACM CSUR 2020/2021).** For representations partitioned by cardinality (VByte, PEF, BIC, PFor, etc.), "the efficiency of intersection is strictly correlated to that of NextGEQ(x) … the efficiency of union is correlated to that of sequential decoding." Roaring and Slicing are partitioned by universe and use neither NextGEQ nor element-by-element merging. Universe-partitioned methods keep their advantage on OR queries as terms increase. For AND, the gap narrows because more selective queries allow larger NextGEQ skips. — [Pibiri & Venturini, "Techniques for Inverted Index Compression," arXiv 1908.10598](https://arxiv.org/pdf/1908.10598)
- **Lucene disjunctions: two algorithms.** Lucene has a heap-based disjunction ("BooleanScorer2"), which pays heap rebalancing on every advance. It also has a bitset-window approach ("BooleanScorer"), which processes the doc-ID space in 2,048-document windows and is often faster with many clauses or high-frequency clauses. Both trace back to Doug Cutting's 1997 "Space Optimizations for Total Ranking." — [Elastic blog](https://www.elastic.co/fr/blog/speedups-top-k-queries-high-frequency-terms); [Lucene BooleanScorer source (8.7)](https://code.yawk.at/org.apache.lucene/lucene-core/8.7.0/org/apache/lucene/search/BooleanScorer.java)
- **Lucene PR #16394 (open in 2026).** It notes that a constant-score disjunction pays a DisiPriorityQueue update on every nextDoc(), and proposes draining 4,096-doc windows in bulk. — [Lucene PR #16394](https://github.com/apache/lucene/pull/16394)
- **Roaring unions.**
  - Array–array union uses a merge (`fast_union_uint16`, SIMD sorting-network merges in the paper) with a heuristic predicting whether the result will become a bitset container.
  - Bitset–bitset union is a vectorized OR with popcount.
  - Wide unions use "lazy" operations that skip cardinality tracking.
  - — [Roaring library paper, SPE 48(4) 2018](https://arxiv.org/html/1709.07821v6); [CRoaring array.c](https://github.com/RoaringBitmap/CRoaring/blob/master/src/containers/array.c)
- **Union on tries.** Arroyuelo & Castillo note that the same trie framework supports unions and differences. Their headline bounds and experiments are for intersection. — [arXiv 2212.00946](https://arxiv.org/pdf/2212.00946)

### Inferences
- Heap-based k-way merge costs O(N log k) for N total postings, a textbook bound not taken from a source read here. Windowed bitset accumulation costs O(N + u/w) per query. That explains why Lucene prefers windows for many or dense clauses and a heap for few or sparse clauses.
- Unlike intersection, union cannot benefit from skipping when the full union is materialized. Practical speed is therefore dominated by decode throughput (the Pibiri–Venturini finding). Dynamic pruning (WAND/BMW, out of scope) is what makes top-k disjunctions fast in practice.
- t-threshold and "minimum-should-match" queries sit between union and intersection. Adaptive theory exists (Barbay–Kenyon) but is incomplete: no lower bound for t < k/2.

### Gaps
- No primary source was read for a tight comparison-model lower bound on k-way union of sorted lists, beyond Demaine et al.'s information-theoretic difficulty measure.
- No primary source was read for PISA's `or_query` implementation. Its use of a simple min-docID scan across cursors is inferred from the code structure of `and_query` and was not verified.
- No recent (2020–2026) primary research specifically on adaptive union or t-threshold algorithms was found.

---

## 5. Connection to worst-case optimal joins and graph set-intersection workloads; 2020–2026 developments

### Takeaway
- Worst-case optimal join (WCOJ) algorithms are generalized multi-way set intersection. Examples: NPRR (PODS 2012), Leapfrog Triejoin (ICDT 2014), Generic Join, Free Join (SIGMOD 2023).
- Their guarantees (the AGM bound, e.g., O(N^1.5) triangles) need each intersection to run in time bounded by the smallest input (the "min property"). That is exactly what galloping or leapfrog intersection provides.
- Beyond-worst-case join theory (Minesweeper, Tetris, box covers) directly generalizes Demaine et al. and Barbay–Kenyon certificates.
- Changes in 2020–2026: WCOJ adoption in systems via hash tries and Free Join; hybrid bitmap layouts for graph intersection (HERO, SIGMOD 2024, up to 188× over merge); tight competitive-ratio lower bounds for set intersection (2025); "fair" composable seekable-iterator intersection (2025).

### Cited Findings
- **Leapfrog Triejoin (Veldhuizen; arXiv Oct 2012, final v5 Dec 2013; ICDT 2014).** Used in the LogicBlox Datalog system. Worst-case optimal in the NPRR sense up to a log factor, and optimal for finer classes, such as instances constrained by projection cardinalities. It can be built on B-trees. The paper gives a class where it runs in O(n log n) while NPRR takes Θ(n^1.375). — [arXiv 1210.0481](https://arxiv.org/abs/1210.0481)
- **The "min property."** Ngo et al. compile a join query into a sequence of set intersections and loops. The worst-case optimal running time holds when each intersection runs in time bounded by the smaller input set. — [EmptyHeaded, arXiv 1503.02368 (SIGMOD 2016)](https://arxiv.org/pdf/1503.02368)
- **EmptyHeaded results.** It lists triangles in O(N^{3/2}), while any pairwise relational-algebra plan needs Ω(N²). It stores sparse sets as sorted uint arrays and dense sets as (offset, bitvector) blocks, and names size disparity "cardinality skew." — [EmptyHeaded](https://arxiv.org/pdf/1503.02368)
- **LevelHeaded (successor).** Reports bitset∩bitset roughly 50× faster than uint∩uint for equal-cardinality sets, and uses relative costs 1 / 10 / 50 for bs∩bs, bs∩uint and uint∩uint in its optimizer. — [LevelHeaded, arXiv 1708.07859](https://arxiv.org/pdf/1708.07859)
- **WCOJ in relational systems (Freitag et al., PVLDB 13, 2020).** Existing WCOJ implementations incur sizable overhead because they rely on ordered index structures on their inputs, or on precomputed read-only indexes as in EmptyHeaded. The paper proposes hash-trie-based WCOJ in a general-purpose RDBMS (Umbra). — [Freitag et al., VLDB 2020](https://www.vldb.org/pvldb/vol13/p1891-freitag.pdf)
- **Free Join (Wang, Willsey, Suciu; SIGMOD 2023, DOI 10.1145/3589295).** WCOJ algorithms have been found less efficient than binary joins on typical acyclic queries. Free Join unifies the two with a new plan type and a data structure merging hash tables and tries. The Rust implementation matches or beats both binary joins and Generic Join. A follow-up, "From Binary Join to Free Join," appeared in SIGMOD Record 53(1), 2024. — [arXiv 2301.10841](https://arxiv.org/pdf/2301.10841); [SIGMOD Record 2024](https://par.nsf.gov/biblio/10518785-from-binary-join-free-join)
- **Beyond-worst-case joins.**
  - Minesweeper (Ngo, Nguyen, Ré, Rudra; PODS 2014) defines certificate complexity that "extends notions of Barbay et al. and Demaine et al." Its running time is linear in certificate plus output for β-acyclic queries, and superlinear for some β-cyclic instances. — [arXiv 1302.0914](https://arxiv.org/pdf/1302.0914)
  - Tetris (arXiv 1404.0703) generalizes this to box certificates over arbitrary indexes, such as dyadic trees. — [arXiv 1404.0703](https://arxiv.org/pdf/1404.0703)
  - Alway, Blais, Salihoglu (ICDT 2021) show that box-cover and certificate sizes depend heavily on domain ordering, and give reordering-based bounds unboundedly better than prior ones on some query families. — [ICDT 2021](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ICDT.2021.3)
- **"Fair intersection of seekable iterators" (Arntzenius, arXiv 2510.26016, Oct 2025).** Extends the seek/leapfrog iterator interface with "bounded interleaving" (positions may be lower bounds), so nested binary intersections compose and match k-way leapfrog behavior. The author explicitly descends from Demaine et al. via Leapfrog Triejoin, frames WCOJ as "a form of fair conjunction," and notes its Seek interface is essentially Kovach et al.'s (2023) indexed streams. One reported micro-benchmark on 30M-element arrays: 1.2 s versus under 1 ms for a 3-way intersection. — [arXiv 2510.26016](https://arxiv.org/html/2510.26016v1)
- **Graph intersection workloads (2020s).**
  - HERO (Yang, Zheng, Lian, Cai, Wang; PACMMOD 2(1), SIGMOD 2024): prior graph set-intersection work mostly follows merge or galloping frameworks with SIMD tuning. HERO's hierarchical bitmap tree (SIB-tree) plus graph reordering reports up to 188× (triangle counting) and 176× (maximal clique enumeration) speedups over classic merge intersection. — [HERO](https://par.nsf.gov/biblio/10533452)
  - Cover-edge triangle counting (arXiv 2403.02997, 2024): relative to a MergePath baseline, forward is 8.6× and hashed-forward variants about 28.7–29.1× faster. Hashing underperforms on road networks, where intersections are tiny. — [arXiv 2403.02997](https://arxiv.org/pdf/2403.02997)
  - "Fast Triangle Counting" (arXiv 2309.09064, 2023) benchmarks merge-path, binary-search, partitioning and hashing variants. Direction-oriented edge orientation gives up to a 2× constant-factor gain. — [arXiv 2309.09064](https://arxiv.org/pdf/2309.09064)
- **Fine-grained lower bounds (2016–2025).** See Q3: 3SUM-based tradeoffs for preprocessed set intersection (KPP SODA 2016; Kopelowitz–Vassilevska Williams ICALP 2020), and a Θ(log n) competitive ratio for deterministic comparison-based intersection (van der Hoog et al., Nov 2025). — [arXiv 1407.6756](https://arxiv.org/abs/1407.6756); [ICALP 2020](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.ICALP.2020.74); [arXiv 2511.02954](https://arxiv.org/abs/2511.02954)
- **Adaptive bounds in compressed space.** Arroyuelo & Castillo show trie-based intersection achieving O(kδ lg(u/δ)), with an ACM version matching Barbay–Kenyon's optimal bound in the pointer-machine model. — [arXiv 2212.00946](https://arxiv.org/pdf/2212.00946); [ACM DL 10.1145/3812648](https://dl.acm.org/doi/10.1145/3812648)

### Inferences
- The posting-list `advance()`/NextGEQ leapfrog loop and the WCOJ inner loop are the same primitive. Improvements in one, such as hybrid bitmap/array layouts or skewed-size heuristics, transfer to the other. EmptyHeaded and LevelHeaded effectively rediscovered Culpepper–Moffat-style hybrid bitvector representations for graphs.
- Through 2026, the main shifts have been systems-level: making WCOJ practical (hash tries, Free Join) and exploiting density (bitmap trees, reordering). No new comparison-model algorithm has displaced galloping/leapfrog for sorted-list intersection. The new theory (competitive-ratio bounds, conditional lower bounds) mostly confirms limits.

### Gaps
- I found no 2020–2026 paper giving a new practical k-way sorted-list intersection algorithm that the IR community has adopted as a replacement for SvS/galloping or leapfrog.
- I did not verify Generic Join's (Ngo–Ré–Rudra, "Skew Strikes Back," 2013) exact statement or its intersection requirement beyond the EmptyHeaded description of the "min property."

---

## 6. De-facto choices in modern search engines and libraries (Lucene, PISA, Roaring), and why

### Takeaway
Production systems converge on the same recipe:
- Order iterators by cost (document frequency).
- Let the rarest list lead.
- Leapfrog the others with `advance(target)`/NextGEQ, implemented by skip data over block-compressed postings (Lucene, PISA) or by galloping inside universe-partitioned containers (Roaring).
- Turn dense filters into bitset membership tests.
- Run disjunctions with a heap or windowed bitsets.

This is SvS/Small_Adaptive-style galloping intersection adapted to compressed, block-skippable lists.

### Cited Findings
- **Lucene ConjunctionDISI (main branch, 2026).**
  - Sub-iterators are flattened and sorted by `cost()` ("Sort the list first to allow the sparser iterator to lead the matching").
  - The two cheapest become `lead1` and `lead2`. `doNext` advances lead2 to lead1's candidate and, on mismatch, advances lead1 to lead2's position (leapfrog). Each remaining iterator is then advanced to the candidate. If one overshoots, lead1 jumps there and the process restarts.
  - BitSetIterators whose cost exceeds the minimum are moved into a `BitSetConjunctionDISI` and applied as O(1) `bitSet.get(doc)` membership tests, with a bulk masking path when the lead is dense ("Bulk masking has fixed per-window cost; sparse leads are cheaper to advance per doc").
  - Two-phase iterators contribute a cheap approximation to the leapfrog. Their expensive `matches()` checks run afterwards, sorted by `matchCost()` ("match cheapest first").
  - — [Lucene ConjunctionDISI.java](https://github.com/apache/lucene/blob/main/lucene/core/src/java/org/apache/lucene/search/ConjunctionDISI.java)
- **Lucene skip data (9.12.0, 28 Sep 2024).** Lucene inlined postings skip data "to improve performance of queries that need skipping such as conjunctions." The Lucene912 postings format embeds Level1SkipData before every 32 packed blocks and Level0SkipData within each packed block. Earlier formats kept skip data in a separate section. — [Lucene 9.12.0 release notes](https://cwiki.apache.org/confluence/display/LUCENE/ReleaseNote9_12_0); [Lucene912PostingsFormat docs](https://lucene.apache.org/core/9_12_2/core/org/apache/lucene/codecs/lucene912/Lucene912PostingsFormat.html)
- **Lucene disjunctions.** Heap-based (DisiPriorityQueue) versus 2,048-document windowed bitset scoring; see Q4. — [Elastic blog](https://www.elastic.co/fr/blog/speedups-top-k-queries-high-frequency-terms)
- **PISA `and_query`.**
  - Sorts cursors "by increasing frequency." The first cursor's docID is the candidate.
  - Each other cursor calls `next_geq(candidate)`. If it lands elsewhere, that docID becomes the new candidate and probing restarts.
  - On a full match, the result is emitted and the lead cursor calls `next()`.
  - `scored_and_query` is identical but sums scores.
  - — [PISA and_query.hpp](https://github.com/pisa-engine/pisa/blob/master/include/pisa/query/algorithm/and_query.hpp)
- **Compressed-list NextGEQ (Pibiri & Venturini).** The Successor query is called NextGEQ in inverted indexes. Moffat & Zobel were first to add skip pointers to compressed lists for faster query evaluation. For cardinality-partitioned codecs, intersection speed tracks NextGEQ speed. — [Pibiri & Venturini, arXiv 1908.10598](https://arxiv.org/pdf/1908.10598)
- **Measured AND times (Pibiri & Venturini).** In Table 12 (ms per AND query), Roaring and Slicing are markedly faster than the cardinality-partitioned codecs. For example, on Gov2 2-term queries, about 0.3 ms for Roaring/Slicing versus about 2–3 ms for VByte/PEF/DINT/QMX, as read from a pdftotext extraction (verify against the PDF). The gap narrows with more terms. Roaring's edge over Slicing is attributed mostly to SIMD. — [Pibiri & Venturini](https://arxiv.org/pdf/1908.10598)
- **Roaring and CRoaring.**
  - Containers: array (≤ 4096 16-bit values), 2^16-bit bitset (> 4096 values), or run container. The type minimizing memory is chosen.
  - Array–array intersection uses galloping when sizes are skewed and a SIMD merge-like intersection otherwise.
  - Bitset operations use SIMD AND/OR plus a Harley–Seal popcount.
  - The authors say: "Use Roaring for bitmap compression whenever possible."
  - Roaring is used in Elasticsearch, Apache Spark and Apache Hive, among others.
  - — [Roaring library paper, SPE 2018](https://arxiv.org/html/1709.07821v6); [arXiv abstract](https://arxiv.org/abs/1709.07821)
- **CRoaring source threshold.** `array_container_intersection` uses `const int threshold = 64; // subject to tuning`. If one cardinality is more than 64× the other, it calls `intersect_skewed_uint16` (galloping, smaller array first). Otherwise it calls `intersect_vector16` (AVX2) or scalar `intersect_uint16`. Array union calls `fast_union_uint16`. — [CRoaring src/containers/array.c](https://github.com/RoaringBitmap/CRoaring/blob/master/src/containers/array.c)
- **Universe- vs cardinality-partitioned layouts.** Universe-partitioned layouts (Roaring, Slicing) skip only at a coarse level (chunks of up to 2^16 integers), so in-chunk costs are always paid even when few results survive. Cardinality-partitioned compressed lists can take large NextGEQ skips on selective multi-term AND queries. — [Pibiri & Venturini](https://arxiv.org/pdf/1908.10598)

### Inferences
- The "why":
  - Leading with the rarest iterator bounds work by the smallest list, which approximates SvS and Small_Adaptive.
  - `advance()` with skip data gives galloping-like O(log gap) or O(1)-block skips on compressed data without full decompression.
  - Bitsets for dense clauses mirror Culpepper–Moffat's hybrid.
  - Experiments (Barbay et al. 2009; Culpepper & Moffat 2010) show these simple strategies beat theoretically stronger round-robin adaptive algorithms on real query logs.
- Lucene's two-lead structure (lead1/lead2 agreement before checking others) is a pragmatic variant of Small_Adaptive. It avoids touching the expensive "others" until the two rarest lists agree.

### Gaps
- No quantitative before/after numbers for Lucene 9.12's inlined skip data on conjunctions were found in primary sources.
- PISA's union (`or_query`) and its block-max variants were not read (block-max variants are out of scope).
- I did not verify Tantivy's or Vespa's intersection implementations.
