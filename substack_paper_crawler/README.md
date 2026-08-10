# Substack paper crawler

Crawls the [recsys.substack.com](https://recsys.substack.com) newsletter
("Top Information Retrieval Papers of the Week"), resolves every featured paper to its
arXiv entry, and downloads it as a PDF.

## Usage

```bash
python3 crawl.py --months 3 --out ../papers      # last 3 months (default)
python3 crawl.py --since 2026-05-08 --out ../papers
python3 crawl.py --free-only --out ../papers     # skip subscriber-only issues
```

No third-party dependencies — standard library only.

| flag | effect |
| --- | --- |
| `--months N` / `--since YYYY-MM-DD` | how far back to crawl (default: 3 months) |
| `--free-only` | only issues published to everyone; most issues are subscriber-only, so this is a much smaller crawl |
| `--refresh` | re-look-up every arXiv ID instead of reusing the ones cached in an existing `manifest.json` |

Re-runs are cheap: arXiv IDs are cached in the manifest and existing PDFs are skipped,
so crawling again costs one API request per *new* paper rather than one per paper.

## Output

```
papers/
  manifest.json                     # machine-readable record of every issue and paper
  INDEX.md                          # human-readable index
  2026-05-08_<issue-slug>/
    01_<Paper_Title>.pdf
    ...
```

## How it works

Each weekly issue lists ~10 papers. Most issues are paywalled, so `body_html` from the
Substack API is truncated — but the truncation happens *after* the table of contents,
and each TOC entry links to a section anchor that is a slugified form of the real paper
title:

```
https://recsys.substack.com/i/208288494/8-the-matryoshka-hypencoder
```

So the pipeline is:

1. **List issues** — `/api/v1/archive`, paginated, filtered by publication date.
2. **Extract papers** — `/api/v1/posts/by-id/<id>`, then:
   - parse the TOC anchors and de-slugify them into titles;
   - for free issues, also read the `<h4>[N] <strong>Title</strong></h4>` headings and
     the `📚 https://arxiv.org/abs/...` link in each section, which are authoritative and
     take precedence over a title lookup.
3. **Resolve** — query the arXiv API by title and accept a hit only when its title,
   compacted to `[a-z0-9]` (which is exactly what slugification preserves), is ≥0.90
   similar to the target. The comparison is what makes the match trustworthy: the
   search itself is fuzzy, the acceptance test is not.
4. **Download** — fetch `arxiv.org/pdf/<id>`, verify the `%PDF` magic bytes, and write
   it under the issue's directory.

### Query fallbacks

Slugification is lossy, which breaks naive title search in two ways the fallback chain
handles explicitly:

- **Boolean collisions** — `and`/`or`/`not` are operators in the arXiv query grammar, so
  a title like *"...Retrieve and Answer Step by Step..."* produces an unparseable query.
  Stopwords are stripped before the query is built.
- **Lost punctuation** — `jina-reranker-v3.5` slugifies to `...v35`, a token that appears
  nowhere in the arXiv index, so every query containing it returns zero hits. Later
  fallbacks search only the longest tokens, which skips these short mangled ones.

Requests to the arXiv API are spaced 3 seconds apart, per its usage guidance.
