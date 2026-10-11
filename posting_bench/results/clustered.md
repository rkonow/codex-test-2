
## Collection and workload

| | |
|---|---|
| collection | synthetic-clustered (u=8000000, terms=20000, max_len=2000000, zipf=1) |
| documents | 8000000 |
| lists / postings | 20000 / 20951573 |
| queries | 500 (mean 2.80 terms, 576257 postings) |
| mean AND / OR result | 309 / 562995 docs |
| CPU | Intel(R) Xeon(R) Processor @ 2.10GHz (sse4.2, avx2, bmi2, avx512f, avx512bw) |

## Summary

| codec | bits/posting | bits/posting (query lists) | build s | decode ns/posting | decode ns/posting (query lists) | best AND ms | best OR ms |
|---|---:|---:|---:|---:|---:|---|---|
| Uncompressed | 32.00 | 32.00 | 0.05 | 0.23 | 0.22 | 0.061 (SvS + galloping) | 1.988 (slices: bitmap) |
| VByte | 10.33 | 9.61 | 0.07 | 1.98 | 1.84 | 0.132 (leapfrog (NextGEQ)) | 3.133 (bitset window (4096)) |
| BP128 | 10.07 | 7.69 | 0.05 | 0.92 | 0.67 | 0.076 (leapfrog (NextGEQ)) | 2.868 (bitset window (4096)) |
| BP128 (bitpacking crate) | 10.07 | 7.69 | 0.03 | 0.72 | 0.57 | 0.071 (leapfrog (NextGEQ)) | 2.677 (min-scan) |
| OptPFor | 7.13 | 5.92 | 0.21 | 1.24 | 0.83 | 0.077 (leapfrog (NextGEQ)) | 2.790 (bitset window (4096)) |
| Elias-Fano | 10.85 | 8.13 | 0.05 | 1.71 | 2.96 | 0.049 (leapfrog (NextGEQ)) | 2.530 (bitset window (4096)) |
| PEF | 7.04 | 4.94 | 0.16 | 1.71 | 1.37 | 0.043 (leapfrog (NextGEQ)) | 3.098 (bitset window (4096)) |
| Interpolative | 5.97 | 4.90 | 0.16 | 4.31 | 4.19 | 0.183 (leapfrog (NextGEQ)) | 4.658 (min-scan) |
| Hybrid (bitmap + BP128) | 10.01 | 7.58 | 0.05 | 1.08 | 0.95 | 0.064 (native) | 1.730 (native) |
| Roaring | 11.72 | 9.32 | 0.13 | 2.30 | 1.59 | 0.028 (native) | 0.486 (native) |
| Trie | 9.61 | 7.18 | 0.58 | 7.39 | 4.62 | 0.126 (native) | 2.456 (native) |
| Run-Trie | 9.13 | 6.67 | 0.70 | 9.14 | 6.17 | 0.134 (native) | 2.567 (native) |
| Roaring (roaring crate) | 11.72 | 9.32 | 0.27 | 8.12 | 7.41 | 0.084 (native) | 4.275 (native) |

## All measurements (ms per query)

| codec | op | algorithm | ms/query |
|---|---|---|---:|
| Uncompressed | AND | SvS + merge | 0.467 |
| Uncompressed | AND | SvS + merge (branchless) | 1.462 |
| Uncompressed | AND | SvS + galloping | 0.061 |
| Uncompressed | AND | SvS + binary search | 0.315 |
| Uncompressed | AND | SvS + Baeza-Yates | 0.098 |
| Uncompressed | AND | SvS + SIMD SSE 4x4 | 0.602 |
| Uncompressed | AND | SvS + SIMD AVX2 8x8 | 0.607 |
| Uncompressed | AND | SvS + SIMD AVX-512 16x16 | 0.497 |
| Uncompressed | AND | SvS + adaptive (SIMD / gallop @64x) | 0.065 |
| Uncompressed | AND | Small Adaptive | 0.075 |
| Uncompressed | AND | Sequential (Barbay-Kenyon) | 0.107 |
| Uncompressed | OR | slices: heap | 7.290 |
| Uncompressed | OR | slices: fold (pairwise merge) | 2.398 |
| Uncompressed | OR | slices: bitmap | 1.988 |
| Uncompressed | AND | leapfrog (NextGEQ) | 0.066 |
| Uncompressed | AND | round-robin (LFTJ) | 0.108 |
| Uncompressed | AND | decode + SvS | 0.240 |
| Uncompressed | OR | heap | 3.465 |
| Uncompressed | OR | min-scan | 2.911 |
| Uncompressed | OR | bitset window (4096) | 2.029 |
| VByte | AND | leapfrog (NextGEQ) | 0.132 |
| VByte | AND | round-robin (LFTJ) | 0.219 |
| VByte | AND | decode + SvS | 0.845 |
| VByte | OR | heap | 4.176 |
| VByte | OR | min-scan | 3.884 |
| VByte | OR | bitset window (4096) | 3.133 |
| BP128 | AND | leapfrog (NextGEQ) | 0.076 |
| BP128 | AND | round-robin (LFTJ) | 0.113 |
| BP128 | AND | decode + SvS | 0.451 |
| BP128 | OR | heap | 4.033 |
| BP128 | OR | min-scan | 2.889 |
| BP128 | OR | bitset window (4096) | 2.868 |
| BP128 (bitpacking crate) | AND | leapfrog (NextGEQ) | 0.071 |
| BP128 (bitpacking crate) | AND | round-robin (LFTJ) | 0.101 |
| BP128 (bitpacking crate) | AND | decode + SvS | 0.287 |
| BP128 (bitpacking crate) | OR | heap | 4.018 |
| BP128 (bitpacking crate) | OR | min-scan | 2.677 |
| BP128 (bitpacking crate) | OR | bitset window (4096) | 2.684 |
| OptPFor | AND | leapfrog (NextGEQ) | 0.077 |
| OptPFor | AND | round-robin (LFTJ) | 0.117 |
| OptPFor | AND | decode + SvS | 0.505 |
| OptPFor | OR | heap | 4.418 |
| OptPFor | OR | min-scan | 2.937 |
| OptPFor | OR | bitset window (4096) | 2.790 |
| Elias-Fano | AND | leapfrog (NextGEQ) | 0.049 |
| Elias-Fano | AND | round-robin (LFTJ) | 0.076 |
| Elias-Fano | AND | decode + SvS | 0.992 |
| Elias-Fano | OR | heap | 4.347 |
| Elias-Fano | OR | min-scan | 3.149 |
| Elias-Fano | OR | bitset window (4096) | 2.530 |
| PEF | AND | leapfrog (NextGEQ) | 0.043 |
| PEF | AND | round-robin (LFTJ) | 0.063 |
| PEF | AND | decode + SvS | 0.733 |
| PEF | OR | heap | 4.409 |
| PEF | OR | min-scan | 3.103 |
| PEF | OR | bitset window (4096) | 3.098 |
| Interpolative | AND | leapfrog (NextGEQ) | 0.183 |
| Interpolative | AND | round-robin (LFTJ) | 0.312 |
| Interpolative | AND | decode + SvS | 2.599 |
| Interpolative | OR | heap | 6.116 |
| Interpolative | OR | min-scan | 4.658 |
| Interpolative | OR | bitset window (4096) | 4.938 |
| Hybrid (bitmap + BP128) | AND | native | 0.064 |
| Hybrid (bitmap + BP128) | OR | native | 1.730 |
| Hybrid (bitmap + BP128) | AND | leapfrog (NextGEQ) | 0.069 |
| Hybrid (bitmap + BP128) | AND | round-robin (LFTJ) | 0.092 |
| Hybrid (bitmap + BP128) | AND | decode + SvS | 0.783 |
| Hybrid (bitmap + BP128) | OR | heap | 5.363 |
| Hybrid (bitmap + BP128) | OR | min-scan | 4.605 |
| Hybrid (bitmap + BP128) | OR | bitset window (4096) | 4.861 |
| Roaring | AND | native | 0.028 |
| Roaring | OR | native | 0.486 |
| Roaring | AND | native, count only | 0.029 |
| Roaring | OR | native, count only | 0.384 |
| Roaring | AND | leapfrog (NextGEQ) | 0.038 |
| Roaring | AND | round-robin (LFTJ) | 0.054 |
| Roaring | AND | decode + SvS | 0.397 |
| Roaring | OR | heap | 4.461 |
| Roaring | OR | min-scan | 3.588 |
| Roaring | OR | bitset window (4096) | 2.710 |
| Trie | AND | native | 0.126 |
| Trie | OR | native | 2.456 |
| Trie | AND | leapfrog (NextGEQ) | 0.179 |
| Trie | AND | round-robin (LFTJ) | 0.239 |
| Trie | AND | decode + SvS | 1.359 |
| Trie | OR | heap | 6.837 |
| Trie | OR | min-scan | 5.201 |
| Trie | OR | bitset window (4096) | 5.530 |
| Run-Trie | AND | native | 0.134 |
| Run-Trie | OR | native | 2.567 |
| Run-Trie | AND | leapfrog (NextGEQ) | 0.194 |
| Run-Trie | AND | round-robin (LFTJ) | 0.267 |
| Run-Trie | AND | decode + SvS | 1.673 |
| Run-Trie | OR | heap | 7.176 |
| Run-Trie | OR | min-scan | 5.926 |
| Run-Trie | OR | bitset window (4096) | 5.608 |
| Roaring (roaring crate) | AND | native | 0.084 |
| Roaring (roaring crate) | OR | native | 4.275 |
| Roaring (roaring crate) | AND | native, count only | 0.084 |
| Roaring (roaring crate) | OR | native, count only | 0.690 |
| Roaring (roaring crate) | AND | leapfrog (NextGEQ) | 0.089 |
| Roaring (roaring crate) | AND | round-robin (LFTJ) | 0.109 |
| Roaring (roaring crate) | AND | decode + SvS | 3.508 |
| Roaring (roaring crate) | OR | heap | 6.054 |
| Roaring (roaring crate) | OR | min-scan | 5.692 |
| Roaring (roaring crate) | OR | bitset window (4096) | 4.839 |
