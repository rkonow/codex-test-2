# Succinct/Compact Data Structures and Learned Approaches for Posting-List Intersection and Union (state as of Oct 2026)

Scope note: this file covers alternatives to classic gap-encoded posting lists: succinct primitives, wavelet trees and self-indexes, treaps, tries, quadtree/ring-style join indexes, and learned/ML methods. Classic codecs (PForDelta, VByte, Roaring internals), SIMD kernels and WAND/BMW internals appear only as comparison baselines. Every result is dated by its publication year. Unless stated otherwise, numbers come from primary papers I read in full text (PDF→text). Where only an abstract or search summary was available, I say so.

## Q1. Rank/select bitvectors and Elias-Fano as succinct primitives; intersection in compressed space with theoretical guarantees

### Takeaway
Elias-Fano (EF) and entropy-compressed rank/select dictionaries (Okanohara & Sadakane 2006/07; Vigna 2012/13) are the basic succinct building blocks. EF gives ≤ 2 + ⌈log(u/n)⌉ bits per element with constant-average-time skipping, which makes it a natural fit for NextGEQ-driven intersection. The strongest recent "compressed space + adaptive time" result is Arroyuelo & Castillo's compact binary tries (arXiv 2022; CPM 2023). They intersect k sets in O(kδ lg(u/δ)) time (δ = Barbay–Kenyon alternation) within space bounded by the trie measure/entropy.

### Cited Findings
- **Vigna, "Quasi-Succinct Indices" (arXiv June 2012):** proposes replacing gap compression in inverted indexes with a quasi-succinct (Elias-Fano) representation of monotone sequences. It claims "expected constant-time operations" and "significant performance improvements on conjunctive, phrasal and proximity queries." — [arXiv 1206.4300](https://arxiv.org/abs/1206.4300)
- **Vigna (2012), EF details:** the high-bits/low-bits split stores the lower ℓ = max{0, ⌊log(u/n)⌋} bits of each element explicitly, using at most 2 + ⌈log(u/n)⌉ bits per element. Skip pointers over the negated-unary upper-bits array give constant-average-time skipping to the first element ≥ a bound. Experiments index TREC GOV2 and compare against MG4J (γ/δ codes), Lucene and Zettair. — [arXiv 1206.4300 (PDF)](https://arxiv.org/pdf/1206.4300)
- **Okanohara & Sadakane, "Practical Entropy-Compressed Rank/Select Dictionary" (arXiv Sept 2006; ALENEX 2007):** introduces four variants (esp, recrank, vcode, sdarray). In practice their size is close to nH0(S), and they claim faster queries than previous implementations. — [arXiv cs/0610001](https://arxiv.org/abs/cs/0610001)
- **sd_vector cost on real posting lists (2022):** `sd_vector<>` uses n lg(u/n) + 2n + o(n) bits. On URL-sorted Gov2/ClueWeb09/CC-News lists it costs 8.45/8.52/7.17 bits per integer (bpi). `rrr_vector<>` costs 11.82/19.94/11.29 bpi because of its o(u) term. Elias-δ gaps cost 3.64/5.40/6.69 bpi. — [Arroyuelo & Castillo, arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Navarro's textbook "Compact Data Structures: A Practical Approach" (Cambridge University Press, Sept 2016, 570 pp.):** the standard reference. Its chapters include Entropy and coding, Arrays, Bitvectors (ch. 4), Permutations (ch. 5) and Sequences (ch. 6). — [Cambridge UP](https://www.cambridge.org/us/universitypress/subjects/computer-science/algorithmics-complexity-computer-algebra-and-computational-g/compact-data-structures-practical-approach); [front matter](https://assets.cambridge.org/97811071/52380/frontmatter/9781107152380_frontmatter.pdf)
- **Adaptive-intersection theory used by the compressed-set work (as summarized in the trie paper, 2022):**
  - Barbay & Kenyon's online algorithm runs in O(δ Σ lg(n_i/δ)) time, where δ is the alternation measure.
  - Demaine, López-Ortiz & Munro's algorithm runs in O(kδ lg(n/δ)) time.
  - Both assume plain sorted arrays of Θ(n_i lg u) bits, which motivates asking whether δ-adaptive time is achievable in compressed space.
  — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Ding & König's intersection structure (as summarized in the trie paper, 2022):** intersects in O(n/√w + k|I(Q)|) time using linear space. Per Ding & König's own experiments, it can be brought to about 1.88× the space of an Elias γ/δ-compressed inverted index. — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Arroyuelo & Castillo, "Trie-Compressed Intersectable Sets" (arXiv Dec 2022) / "Trie-Compressed Adaptive Set Intersection" (CPM 2023, LIPIcs 259):**
  - A sorted set S ⊆ [0..u) of n elements is stored in compressed space while supporting k-way intersection in adaptive O(kδ lg(u/δ)) time.
  - The algorithm is Trabb-Pardo's classic trie intersection, implemented on compact (BFS, level-wise bitvector) binary tries with rank.
  - Space is bounded by the Klein–Shapira trie entropy, plus a run-length variant (rTrie) that collapses full subtrees.
  — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946); [CPM 2023 (Dagstuhl)](https://drops.dagstuhl.de/entities/document/10.4230/LIPIcs.CPM.2023.1)
- **Follow-up, Alanko, Becker, Cenzato, Gagie, Kim, Kodric & Prezza, "The Trie Measure, Revisited" (arXiv Apr 2025; CPM 2025):**
  - Studies prefix-free re-encodings of the universe that minimize the total edges of the binary tries of n sets. The general problem is NP-hard (equivalent to the cheapest sequence of binary unions).
  - Gives O(u + N log u) and O(N log² u) algorithms for the best "shifted" encoding, and O(N + u³) for the best ordered encoding.
  - Experiments use 13 set-sequence datasets (not web inverted indexes). Optimal vs. worst shift differ by >5% on only 5 datasets (max 15.85%). Optimal shifted-ordered encodings are >10% smaller on 7 datasets (peak 27.31%).
  - The paper explicitly cites offline set intersection (Arroyuelo & Castillo) as an application.
  — [arXiv 2504.10703](https://arxiv.org/abs/2504.10703)

### Inferences
- EF/sd_vector is the practical succinct primitive behind PEF. The 2022 tries show that a structure whose space tracks the trie measure (3.48–5.18 bpi measure on these collections) rather than the gap measure (2.25–3.70 bpi) can trade some space for δ-adaptive, navigation-based intersection.
- The 2025 trie-measure work suggests that universe re-encoding (docID shifts/orderings) is a lever for shrinking trie-based set representations. That is analogous to docID reordering for gap codecs, but it has not been evaluated on web-scale posting lists.

### Gaps
- I could not locate or verify the Barbay–Gupta–Rao–Sorenson papers named in the brief ("Dynamic online multiselection", "Synergistic sorting and compression"), nor a specific Barbay–Navarro paper on "compressed representations for set operations." A search surfaced only related Barbay–Navarro work on compressed permutations and binary relations, without intersection-specific results ([Barbay publications list](https://users.dcc.uchile.cl/~jbarbay/Publications/listOfPublications-Barbay.pdf)). These are not reported as findings.
- The CPM 2023 camera-ready numbers were not checked; all trie numbers here are from arXiv v1 (Dec 2022).

## Q2. Wavelet trees / wavelet matrices and self-indexes for document retrieval and intersection

### Takeaway
Wavelet trees let a single structure answer both docID-ordered and weight-ordered access, plus "native" range intersection. Dual-sorted inverted lists (Navarro & Puglisi 2010; Konow & Navarro 2012) were the fastest option for ranked conjunctive queries in 2012-era comparisons, at about half the space of keeping two separately ordered indexes. Full-text self-indexes (FM-index/CSA-based document listing) cost very little space on top of the compressed text, but are orders of magnitude slower than inverted indexes for word queries.

### Cited Findings
- **Gagie, Navarro & Puglisi, "New Algorithms on Wavelet Trees and Applications to Information Retrieval" (TCS 426–427:25–41, 2012; arXiv Nov 2010; early parts at SPIRE 2009/2010):** shows how wavelet trees solve range quantile, range next-value and range intersection queries. Applications include document retrieval on hierarchical and temporal documents, and the representation of inverted lists. — [Navarro abstract page](https://users.dcc.uchile.cl/~gnavarro/abstracts/tcs11.2.html); [arXiv 1011.4532](https://arxiv.org/abs/1011.4532)
- **Navarro & Puglisi, "Dual-Sorted Inverted Lists" (SPIRE 2010):** a wavelet-tree technique that keeps one structure with the combined functionality of two orderings (decreasing term frequency for ranked retrieval, increasing docID for phrase/Boolean queries) "within the space of one compressed inverted index." It supports the primitives needed for any intersection algorithm, plus space-free stemming and hierarchical documents. — [Navarro abstract page](https://users.dcc.uchile.cl/~gnavarro/abstracts/spire10.2.html)
- **How dual-sorted works (as described in Inverted Treaps, TOIS 2017):**
  - Lists are stored sorted by decreasing frequency in a wavelet tree, which simulates increasing-docID order.
  - Approximate ranked unions use term-at-a-time (TAAT) processing; exact ranked intersections use a document-at-a-time (DAAT) style.
  - Intersection is done "in native form on wavelet trees … even faster than Block-Max": the wavelet tree recursively subdivides the docID universe and detects when a list has no documents in an interval.
  - Limitations: dual-sorted does not know frequencies until it reaches leaves, and it cannot differentially encode docIDs.
  — [Konow et al., Inverted Treaps (author PDF)](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Konow & Navarro, "Dual-Sorted Inverted Lists in Practice" (SPIRE 2012, LNCS 7608:295–306):**
  - Datasets: a 165 GB / 14.4M-document GOV2 sample and a 33.2 GB / 11.8M-document English Wikipedia.
  - Dual-sorted was the fastest alternative for top-20 conjunctive queries, beating Block-Max, a docID-sorted inverted index and Zettair.
  - For bag-of-words queries it is slower than a frequency-sorted index but better than their Block-Max implementation.
  - It handles both query types within 10–15% extra space over the plain collection size (the state-of-the-art cost for one query type), i.e., "about half the space of the sum of a docid-sorted and a frequency-sorted index."
  — [SPIRE 2012 PDF](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.5.pdf); [abstract](https://users.dcc.uchile.cl/~gnavarro/abstracts/spire12.5.html)
- **Context from the same 2012 paper on word-based self-indexes:**
  - Wavelet-tree-based self-indexes (Brisaboa et al.; Arroyuelo et al. SPIRE 2010 for Boolean AND/OR) need about 3% extra on top of the compressed text, versus 15–20% for inverted indexes. Within that niche they are "unbeaten," but their query times are "orders of magnitude slower" than explicit inverted indexes.
  - Culpepper et al.'s document-array wavelet tree uses 200–400% of text size (30–60% if restricted to words), but supports arbitrary substrings.
  — [SPIRE 2012 PDF](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.5.pdf)
- **Claude & Navarro, "The Wavelet Matrix" (SPIRE 2012):** a level-wise rearrangement of wavelet-tree bitmaps that is simpler to build and query. It roughly doubles (or more) access speed compared with the level-wise wavelet tree, so applications built on level-wise wavelet trees "will become up to twice as fast." — [SPIRE 2012 PDF](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.4.pdf)
- **Arroyuelo, González & Oyarzún, "Compressed Self-Indices Supporting Conjunctive Queries on Document Collections" (SPIRE 2010, pp. 43–54).** The same group also published Arroyuelo, Gil-Costa, González, Marín & Oyarzún, "Distributed Search based on Self-Indexed Compressed Text" (IPM 48(5):819–827, 2012). Titles and venues are confirmed from the author's publication list; I did not retrieve their results. — [Arroyuelo publications](https://users.dcc.uchile.cl/~darroyue/pubs.htm)
- **Navarro, "Spaces, Trees and Colors: The Algorithmic Landscape of Document Retrieval on Sequences" (ACM CSUR 46(4), Article 52, 2014):** the survey of self-index-based document listing and top-k retrieval for general sequence collections. It notes that inverted indexes remain preferred for natural-language text. — [arXiv 1304.6023](https://arxiv.org/abs/1304.6023); [abstract page](https://users.dcc.uchile.cl/~gnavarro/abstracts/acmcs14.html)

### Inferences
- The wavelet-tree line wins mainly when one structure must serve several orderings or query types (ranked, Boolean, phrase, substring). For pure docID-ordered Boolean AND on web text, later work (treaps 2013/2017, tries 2022) supersedes it.
- The wavelet-tree "native intersection" idea (prune docID intervals where some list is empty) is conceptually the same pruning that later reappears in treaps (with frequency bounds) and binary tries (empty subtrees).

### Gaps
- I found no 2018–2026 head-to-head of dual-sorted or wavelet-tree inverted lists against modern PISA baselines (PEF/VBMW/MaxScore).
- I did not retrieve recent r-index or other repetitive-collection document-listing numbers. These matter mostly for versioned or repetitive collections, not web search.

## Q3. Tree-based compressed representations: inverted treaps and ranked intersection/union vs. Block-Max WAND

### Takeaway
Inverted treaps (SIGIR 2013; TOIS 2017) store each posting list as a treap: docID is the BST key and term weight is the heap priority. Topology is encoded with succinct trees, and docIDs and weights are both differentially encoded. The heap order acts as a hierarchical block-max: subtree roots bound all weights below them, so ranked AND/OR can prune and threshold in one pass. Under tf-idf (GOV2, 2014-era baselines), treaps use about Block-Max's space and run ranked unions 3–10× faster than Block-Max and ranked intersections up to 2× faster for k ≤ 100. Under quantized BM25, their space and speed advantages shrink.

### Cited Findings
- **Konow, Navarro, Clarke & López-Ortiz, "Faster and Smaller Inverted Indices with Treaps" (SIGIR 2013, pp. 193–202):** the conference version reported about 20% less space and up to 3× faster queries. — [abstract page](https://users.dcc.uchile.cl/~gnavarro/abstracts/sigir13.html); [PDF](https://users.dcc.uchile.cl/~gnavarro/ps/sigir13.pdf)
- **"Inverted Treaps" (ACM TOIS, DOI 10.1145/3007186, print 2017), abstract:** the treap allows "intersect/merge the document identifiers while simultaneously thresholding by frequency, instead of the costlier two-step classical processing methods." The treap topology uses alternative compact data structures, and both docIDs and frequencies are differentially encoded. — [author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf); [Crossref](https://api.crossref.org/works/10.1145%2F3007186)
- **Relation to Block-Max (TOIS 2017):** "Our technique can be seen as a generalization of the Block-Max idea, in which we use the treap concept to naturally define a hierarchical blocking scheme … it is practical and beats the flat Block-Max." Compared with dual-sorted, treaps carry an upper bound on frequencies in each docID interval, so ranked intersections run faster than dual-sorted's Boolean intersections. Treaps are also smaller, because dual-sorted cannot differentially encode docIDs. — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Implementation (TOIS 2017):**
  - Built on sdsl-lite. DocIDs are stored in a DAC vector (width 6) and weights in a DAC vector (width 2).
  - Postings with frequency below a threshold f0 are split off into separate PForDelta "f0 lists," one per frequency.
  - Lists with fewer than 1024 elements are stored in WAND format.
  - Three topology encodings: LOUDS (smallest), BP, and HEAP (fastest, about 2× LOUDS's topology size).
  - Under tf-idf, the topology is about 7% of total index size.
  — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Setup (TOIS 2017):**
  - Collection: TREC GOV2 (25.2M documents, 4.9B postings).
  - Queries: 48,583 TREC 2005/2006 efficiency queries.
  - Baselines: Elias-Fano WAND (Ottaviano–Venturini code), block-compressed WAND (OptPFD docIDs + Simple9 frequencies, 128-blocks), Block-Max (BMAX), Dual-Sorted, and ATIRE (impact-sorted).
  - Results on Wikipedia and Weblogs "do not change significantly."
  — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Space (TOIS 2017):**
  - EF is clearly the smallest index, and WAND is about 10% larger.
  - Under tf-idf, the fastest treap is about 25% larger than WAND, the same size as Block-Max, and 15% smaller than Dual-Sorted; the LOUDS treap is about 10% smaller than BMAX.
  - Under quantized BM25, treaps are 35–40% larger than Block-Max/Dual-Sorted, growing from about 13% to 18% of text size. The cause: far fewer postings have weight 1 (3 billion under tf-idf vs. 800 million under BM25), which shrinks the f0-list savings.
  — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Time under tf-idf (TOIS 2017):**
  - Ranked unions: 10× (k=10) to 3× (k=1000) faster than Block-Max, and 45–50× to 6–7× faster than WAND and Dual-Sorted.
  - Ranked intersections: 2× faster than Block-Max at k=10, converging to similar times at k=1000. Versus WAND, from 2× to 40% faster; versus Dual-Sorted, from 80% faster to 10% slower.
  - The treap is always the fastest index up to k=100, and always fastest for one- and two-word queries.
  — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Time under quantized BM25 (TOIS 2017):** treaps are "slightly better than all the alternatives on ranked unions." Ranked intersections range from 2× faster (k=10) to 15% slower (k=1000), and treaps are still fastest at k=100. — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **One-word queries (TOIS 2017):**
  - The treap is 20× faster than Dual-Sorted and 25–200× faster than Block-Max under tf-idf (40–300× under BM25).
  - Rough fits: HEAP treap 0.15k + 6.5 µs (tf-idf); Dual-Sorted 4k + 100 µs.
  - The impact-sorted ATIRE is "clearly the fastest alternative for single-term queries" at about 0.05k + 1 µs.
  — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Updates (TOIS 2017):** a dynamized (incremental) variant costs 50–100% overhead in space, construction and query time over the static treaps. — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)

### Inferences
- Treaps win most clearly on ranked disjunctive (OR) queries with small k under scoring schemes with many low-weight postings (tf-idf). Their benefit narrows with BM25 impacts and large k, and impact-ordered indexes still win single-term top-k.
- The baselines are 2013–2014 implementations (Block-Max with OptPFD, EF-WAND without block-max). There is no published comparison against later Variable BMW or PISA's MaxScore/VBMW over PEF. Today's margins are therefore unknown and likely smaller.

### Gaps
- I found no 2018–2026 re-evaluation of inverted treaps against modern dynamic-pruning baselines, or on ClueWeb12/MS MARCO/learned-sparse indexes.

## Q4. Compressed representations designed for set operations (hybrid run-length, tries, k²-tree/quadtree, connections to worst-case-optimal joins)

### Takeaway
Since 2013, Arroyuelo and colleagues have exploited docID reordering. Run-length/gap hybrids (SIGIR 2013, IPM 2018) and compact binary tries (2022/2023) turn the runs and clusters that reordering creates into faster AND/OR. The tries are 1.55–3.32× faster than PEF at 1.15–1.72× PEF's space, and both smaller and faster than Roaring. The same compact-structure toolbox (wavelet trees, compressed quadtrees/qdags) underlies worst-case-optimal (WCO) multiway joins in compact space: the Ring (SIGMOD 2021; TODS 2024) and qdags (TODS 2022). These are the database analogue of multi-term conjunctive queries.

### Cited Findings
- **Arroyuelo, Oyarzún, González & Sepúlveda, "Hybrid compression of inverted lists for reordered document collections" (IPM 54(6):1308–1324, 2018):**
  - Introduces a docID reordering that creates runs of consecutive docIDs in selected lists, plus hybrid schemes mixing gap and run-length encodings.
  - Reports about 10–30% less docID space than the best prior methods.
  - Decompression is up to 1.22× faster when runs are decompressed explicitly and up to 4.58× faster with implicit decompression. AND, WAND and full-OR query times also improve.
  - These figures come from the abstract; the full text was not read.
  — [Univ. de Chile repository](https://repositorio.uchile.cl/handle/2250/169432)
- **Predecessor:** Arroyuelo, González, Oyarzún & Sepúlveda, "Document identifier reassignment and run-length-compressed inverted indexes for improved search performance" (SIGIR 2013, pp. 173–182). — [Arroyuelo publications](https://users.dcc.uchile.cl/~darroyue/pubs.htm)
- **Trie experiments, setup (Arroyuelo & Castillo, arXiv 2022 / CPM 2023):**
  - Gov2 and ClueWeb09 indexes from Lemire's URL-sorted docID release with 20,000 TREC Million-Query queries; CC-News from Mackenzie et al.'s CIFF index with 9,666 queries.
  - Only lists of length ≥ 4096 are kept.
  - Baselines come from PISA (IPC, Simple16, OptPFD, PEF-Opt, VarintGB, Varint-G8IU, SIMD-BP128), plus CRoaring and RUP.
  — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Trie experiments, Table 2 (average intersection time in ms / space in bpi):**

  | Index | Gov2 | ClueWeb09 | CC-News |
  |---|---|---|---|
  | PEF-Opt | 1.88 ms / 3.62 bpi | 6.50 / 5.85 | 17.33 / 5.80 |
  | OptPFD | 2.15 / 4.07 | 7.79 / 6.28 | 11.80 / 6.50 |
  | Roaring | 1.09 / 8.77 | 3.75 / 12.62 | 5.56 / 9.86 |
  | RUP | 1.10 / 5.04 | 4.27 / 8.44 | 5.44 / 8.41 |
  | SIMD-BP128 | 1.29 / 6.07 | 4.47 / 8.98 | 15.90 / 7.36 |
  | rTrie(v) | 0.77 / 4.81 | 1.96 / 7.96 | 6.09 / 9.95 |
  | rTrie(v5) | 1.22 / 4.22 | 3.07 / 6.95 | — |
  | trie(v) | 0.77 / 5.85 | 1.64 / 8.50 | 5.21 / 9.99 |

  — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Trie experiments, headline ratios:** tries compute intersections 1.55–3.32× faster than PEF at 1.15–1.72× its space; 1.07–1.91× faster than Roaring at 0.48–1.01× its space; and 1.04–2.18× faster than RUP at 0.82–1.19× its space. On CC-News, which has fewer runs, the advantage shrinks: trie uses 0.889–1.013× Roaring's space and is only up to 1.067× faster. Code: github.com/jpcastillog/compressed-binary-tries. — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Ring, Arroyuelo, Gómez-Brandón, Hogan, Navarro, Reutter, Rojas-Ledesma & Soto (SIGMOD 2021 "Worst-Case Optimal Graph Joins in Almost No Space"; extended in ACM TODS 2024):**
  - Each triple is treated as a cyclic string; the columns of its sorted rotations are stored in wavelet trees.
  - One ring replaces the 6 index orders that B-trees or tries need for all WCO joins, using sublinear extra space on top of the graph.
  - On a 1-billion-edge Wikidata subgraph: 8% extra space over the raw integer data; 4–6× less space than Jena, RDF-3X, Virtuoso and Blazegraph while 2–6× faster on basic graph patterns.
  - Jena LTJ uses 10× and EmptyHeaded 160× more space; only Jena LTJ is faster, by about 20% on average.
  - Qdag is smaller but "hundreds of times slower overall." A compressed ring reaches 60% of the ring's space (matching Qdag) while running 40% slower.
  — [TODS 2024 author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tods24.pdf); [TODS abstract](https://users.dcc.uchile.cl/~gnavarro/abstracts/tods24.html); [SIGMOD 2021 abstract](https://users.dcc.uchile.cl/~gnavarro/abstracts/sigmod21.html)
- **Qdags, Arroyuelo, Navarro, Reutter & Rojas-Ledesma, "Optimal Joins using Compressed Quadtrees" (ACM TODS 2022):**
  - Relations are point sets in d-dimensional grids stored as compressed quadtrees (k²-tree-like). Intermediate results are qdags (quadtrees that share subtrees).
  - The join algorithm is WCO in data complexity. Lazy qdags extend it to unions and negations.
  - The index uses "even less space than what is needed to store the data in raw form," and one to two orders of magnitude less than other WCO indexes, while being "competitive in time."
  — [TODS 2022 abstract](https://users.dcc.uchile.cl/~gnavarro/abstracts/tods22.html)

### Inferences
- The pruning mechanism shared by binary tries, quadtrees/qdags and wavelet trees (recursively partition the universe and stop at empty sub-ranges) is exactly what gives δ-adaptive or WCO behavior. Multi-term conjunctive search is the 1-dimensional special case, so qdag/ring results are conceptual evidence that compact representations need not cost speed for multiway intersections.
- The trie results depend heavily on clustered (URL-sorted) docIDs and long lists (≥ 4096). On CC-News, with fewer runs, tries roughly tie Roaring and fall behind PEF on space.

### Gaps
- I found no paper titled "qk²-trees" or "Compressed k²-tree-based intersections" by Arroyuelo et al. The closest verified works are qdags (compressed quadtrees, TODS 2022) and "Faster Dynamic Compressed d-ary Relations" (SPIRE 2019, title only, from the [Arroyuelo publications page](https://users.dcc.uchile.cl/~darroyue/pubs.htm)).
- I found no 2024–2026 journal extension of the trie work with union or ranked (top-k) support; the tries paper covers intersection only.

## Q5. Learned and ML-based approaches (learned indexes for postings, learned compression, learned Bloom filters, 2023–2026)

### Takeaway
Learned methods for posting lists have moved from feasibility estimates (Oosterhuis, Culpepper & de Rijke, ADCS 2018) to learned rank/select (LA-vector, ALENEX 2021 / TALG 2022), generic learned compression (LeCo, SIGMOD 2024), and now a dedicated learned inverted-index codec with SIMD AND/OR operators (LICO, SIGMOD 2026). LICO is the 2026 state of the art for learned posting compression. It sits on the size/latency Pareto frontier and reports 2× lower intersection latency and about 4–5× lower union latency than PEF/OptPFor, but it does not beat PEF/Rice/BIC on space. I found no learned or neural method for ranked set operations or learned intersection caching in 2023–2026.

### Cited Findings
- **Oosterhuis, Culpepper & de Rijke, "The Potential of Learned Index Structures for Index Compression" (ADCS 2018):**
  - Uses learned models acting as learned Bloom filters (term × document membership) for conjunctive Boolean search, including a two-tier scheme (truncated lists plus remainder) where results are guaranteed correct if at least one query term's list is untruncated.
  - On Robust04, GOV2 and ClueWeb09B with OptPFOR, fewer than 1% of terms account for 40% of index storage.
  - Estimated storage gain on Robust is ≥ 40% with truncated length 4,000 while replacing fewer than 4,000 term lists. Model cost is bounded at 0 to 512 bits per document and per term (a 128-dimensional compressed embedding).
  - This is a space estimate only; no trained model or timings are given.
  — [arXiv 1811.06678](https://arxiv.org/abs/1811.06678); [ACM DL](https://dl.acm.org/doi/10.1145/3291992.3291993)
- **Boffa, Ferragina & Vinciguerra, LA-vector (ALENEX 2021; extended in ACM TALG 2022):**
  - The first compressed rank/select dictionary that models integers as points and uses piecewise-linear approximations, with bounds in the worst case and for finite mean/variance inputs.
  - Tested on Web search, DNA, IR and NLP datasets against Elias-Fano, RRR and γ/δ-coded gap vectors. Its select is the fastest and its rank is on the space–time Pareto frontier.
  - These claims come from the publication pages; the full text was not read because the site's TLS certificate had expired.
  — [ALENEX 2021 page](https://learned.di.unipi.it/publication/learned-rank-select/); [TALG page](https://learned.di.unipi.it/publication/learned-approach-to-design-compressed-rank-select-data-structures)
- **Liu et al., "Learned Data Compression: Challenges and Opportunities for the Future" (arXiv Dec 2024, vision paper):**
  - Admits that la-vector "cannot show dominant performance in practice" against OptP4Delta and BIC.
  - On CC-News (avg bits/int; decode ns/int): BIC 7.38 bits/int, 10.41 ns/int; OptPFor 8.30, 1.32; la-vector 8.84, 0.81; SIMD la-vector 8.84, 0.57 (6.535 GiB/s, 1.68× OptP4Delta and 18.25× BIC throughput).
  - Argues that each ε-PLA segment bounds its key range, so segments act as skip pointers "without incurring extra space costs" for AND/OR.
  — [arXiv 2412.10770](https://arxiv.org/abs/2412.10770)
- **Zhu, Liu, Zhang, Sha, Liao, Hu & Chen, "LICO: An SIMD-Aware High-Performance Learned Inverted Index Compression Framework" (Proc. ACM Manag. Data 4(3), SIGMOD 2026, Article 202, June 2026):**
  - Encodes posting lists with error-bounded ε-PLA models plus residual arrays. It auto-tunes ε with a cost model and partitions lists greedily (4 orders of magnitude faster than DP).
  - The space model is 0.5·n·log2 σ² + O(n) bits, where σ² is the gap variance.
  - Ships SIMD (AVX-512, including vp2intersect) decode, intersection and union. Code: github.com/xianyuzhuruc/LICO.
  — [ACM DL](https://dl.acm.org/doi/10.1145/3802079); [open-access PDF](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
- **LICO setup (2026):**
  - Datasets with URL-sorted docIDs: CW12B (2,289 lists, 8.7B docIDs), CCNews (3,066 lists, 13.9B), WITD (899 lists, 0.31B).
  - Baselines: VByte, OptVByte, BIC, Delta, Rice, PEF, DINT, FastPFor, OptPFor, Simple16, QMX, LA-vector, LeCo-var.
  - LA-vector and LeCo are excluded from the query experiments because they lack native list operators. Roaring and trie/treap structures are not compared.
  — [LICO PDF](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
- **LICO space and random access (2026):**
  - PEF and Rice achieve the best compression, followed by LICO++ and OptPFor. LICO++ improves on LICO's compression by up to 1.29×.
  - LA-vector and LeCo are less space-efficient: LA-vector "does not capture fine-grained locality," and LeCo is "not optimized for inverted index compression."
  - Average fit across the three datasets: LICO-OptimalPLA 7.15 bits/int, 0.61 ns/int decoding.
  - Random access: PEF is best at 7.17 ms/query average, vs. LICO++ 7.60 and LICO 8.88.
  — [LICO PDF](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
- **LICO intersection (2026), average ms/query:** LICO 33.67 vs. PEF 71.17, QMX 59.57, OptPFor 72.89 on CW12B; 36.67 vs. PEF 72.67 on CCNews; 1.97 vs. PEF 4.35 on WITD. That is up to 2.64× faster than the fastest non-learned SIMD codec. LICO++ degrades with 5+ terms because its larger ε weakens NextGEQ pruning. — [LICO PDF](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
- **LICO union (2026), average ms/query:** LICO 48.67 vs. DINT 218.52 and PEF 262.30 on CW12B; 56.42 vs. 242.42 / 290.48 on CCNews. The overall claim is up to 5.52× query speedup. — [LICO PDF](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
- **LeCo, "Lightweight Compression via Learning Serial Correlations" (SIGMOD 2024, PACMMOD 2(1), Article 65):**
  - A general learned "serial correlation" compressor; FOR, Delta and RLE are special cases.
  - Up to 91% better compression ratio than FOR/Elias-Fano with comparable decompression and random-access speed.
  - Up to 5.2× faster analytical queries in Arrow and 16% higher RocksDB throughput.
  - Evaluated on columnar and KV data rather than posting lists.
  — [arXiv 2306.15374](https://arxiv.org/abs/2306.15374)
- **Ferragina & Lari, "Compressibility Measures and Succinct Data Structures for Piecewise Linear Approximations" (arXiv Sept 2025; ISAAC 2025):** "the first theoretical analysis of the maximum compressibility achievable by PLA-based learned data structures." Gives lower bounds and near-matching succinct PLA encodings; theory only, no experiments. — [arXiv 2509.07827](https://arxiv.org/abs/2509.07827)
- **Learned Bloom filters (2024–2025):** recent work targets generic membership or LSM key-value stores, not posting lists. Examples: Cascaded Learned Bloom Filter (arXiv Feb 2025), and learned LSM-trees reporting up to 2.28× lower GET latency and 70–80% less filter memory (arXiv Jul 2025). — [arXiv 2502.03696](https://arxiv.org/abs/2502.03696); [arXiv 2508.00882](https://arxiv.org/abs/2508.00882)

### Inferences
- The learned-compression gains on posting lists come from the same property as the trie and run-length gains: local linearity or clustering of URL-sorted docIDs (LICO's average PLA segment length is 27–307 keys depending on ε). Gains on randomly ordered docIDs are untested.
- LICO's union and intersection wins are partly a SIMD/hardware story: AVX-512 vp2intersect against scalar or partially SIMD baselines, including PEF. Its learned component mostly provides skip-pointer-free pruning and fast linear decode, not better compression.
- Learned methods are so far limited to Boolean set operations and decoding. None supports ranked (top-k) AND/OR with score bounds as treaps or block-max do.

### Gaps
- I found no 2023–2026 learned or neural method that learns the intersection itself, learned caching of intersections for posting lists, or learned Bloom filters deployed for posting-list membership in search engines.
- LICO's latencies (tens of ms) reflect only the long lists in its benchmark (about 2–3K lists per collection). End-to-end search-engine query latency comparisons are not reported.
- No published comparison exists between LICO and the compact tries, Roaring, or treaps.

## Q6. Reported space/time tradeoffs vs. state-of-the-art inverted indexes (PEF, OptPFD, Roaring): when do the alternatives win?

### Takeaway
Across the primary papers, compact and learned alternatives rarely beat PEF on space. Their wins are in time:
- Tries: 1.55–3.32× faster AND than PEF at 1.15–1.72× its space; smaller and faster than Roaring.
- LICO: about 2× faster AND and about 4–5× faster OR than PEF with AVX-512, at slightly larger space.
- Treaps: 2–10× faster ranked OR/AND than 2014-era Block-Max at Block-Max space under tf-idf.

These wins depend on docID reordering (runs, clusters), long lists, small k for ranked queries, and tf-idf-like weight distributions.

### Cited Findings
- **Space reference points on URL-sorted lists of length ≥ 4096 (2022):** PEF-Opt 3.62/5.85/5.80 bpi and Roaring 8.77/12.62/9.86 bpi on Gov2/ClueWeb09/CC-News; tries 4.22–5.85 bpi on Gov2. — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Tries win on time against all tested baselines on Gov2 and ClueWeb09:** rTrie is 1.549–2.442× (Gov2) and 2.117–3.316× (ClueWeb09) faster than PEF at 1.166–1.361× its space. It uses 0.481–0.631× Roaring's space while up to 1.913× faster. On CC-News (fewer runs) the margin narrows. trie uses 1.512–1.722× PEF's space while 1.987–3.326× faster. Against Roaring it uses 0.889–1.013× the space and is at most 1.067× faster. Against RUP it uses 1.043–1.188× the space and is at most 1.044× faster. — [arXiv 2212.00946](https://arxiv.org/abs/2212.00946)
- **Treaps vs. dynamic pruning (TOIS 2017):** they win ranked unions at all k ≤ 1000 and ranked intersections up to k = 100 under tf-idf, at about Block-Max space (EF-WAND remains smallest). Under BM25 they need 35–40% more space and lose at k = 1000 on intersections. Impact-sorted ATIRE wins single-term queries. — [TOIS author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tois16.pdf)
- **Dual-sorted (2012):** wins ranked conjunctive queries and saves about 50% of space when both docID-ordered and frequency-ordered access are needed. It loses bag-of-words speed to a frequency-sorted index. — [SPIRE 2012 PDF](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.5.pdf)
- **Word-based self-indexes:** about 3% extra over compressed text (vs. 15–20% for inverted indexes), but orders of magnitude slower. They win only when space is paramount or when substring/phrase functionality is needed. — [SPIRE 2012 PDF](https://users.dcc.uchile.cl/~gnavarro/ps/spire12.5.pdf)
- **LICO (2026):** about 2× faster intersection than PEF and OptPFor, and 4–5× faster union than PEF or DINT, on CW12B/CCNews/WITD. PEF and Rice remain more compact, and PEF has the best random access. — [LICO PDF](https://upenn.ferric.systems/dbscholar/sigmod_papers/2026/10.1145_3802079.pdf)
- **Hybrid run-length plus gap encoding on reordered collections (2018):** 10–30% smaller docID space than the best prior methods, with faster AND, WAND and OR (abstract-level claim). — [Univ. de Chile repository](https://repositorio.uchile.cl/handle/2250/169432)
- **Multiway joins (2021–2024):** compact WCO indexes (Ring) use 8% over raw data and are 2–6× faster than conventional RDF stores. Compressed quadtrees (qdags) are smaller than raw data, but much slower than the Ring overall. — [TODS 2024 author PDF](https://users.dcc.uchile.cl/~gnavarro/ps/tods24.pdf); [TODS 2022 abstract](https://users.dcc.uchile.cl/~gnavarro/abstracts/tods22.html)

### Inferences
- **Win conditions:** the alternatives win when (a) docIDs are reordered so lists contain runs and dense clusters (tries, run-length hybrids, learned PLA), (b) lists are long (both the tries and LICO filter to long lists), (c) the workload is OR-heavy or ranked with small k (treaps), or (d) one structure must serve several orderings or query types (dual-sorted, self-indexes, Ring).
- **Where PEF and other gap codecs still win:** space for typical lists, random access, and robustness to unclustered docIDs and BM25-like weight distributions.
- **Comparability:** results from different papers use different hardware, list filters and query logs (TREC 2005/06 efficiency, TREC 1MQ, CC-News CIFF). Cross-paper speed ratios are not comparable; only within-paper ratios are reliable.

### Gaps
- No single benchmark compares PEF, Roaring, tries, treaps and LICO on the same hardware and query set.
- No 2023–2026 evaluation of compact or learned set structures on learned-sparse (SPLADE-style) indexes was found in this search.
- Ranked-query results for tries or learned codecs are absent.
