#!/usr/bin/env python3
"""Crawl papers featured in the recsys.substack.com newsletter and save them as PDFs.

The newsletter ("Top Information Retrieval Papers of the Week") publishes a weekly
issue listing ~10 papers. Most issues are paywalled, but every issue -- paid or free --
exposes a table of contents whose anchor links are slugified versions of the real
paper titles, e.g.

    https://recsys.substack.com/i/208288494/8-the-matryoshka-hypencoder

That is enough to recover the title, look the paper up on the arXiv API, and fetch
the PDF. Free issues additionally expose the arXiv URL directly in the body, which
is used in preference to a title lookup when available.

Usage:
    python crawl.py --months 3 --out ../papers
"""

from __future__ import annotations

import argparse
import html
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field, asdict
from datetime import datetime, timedelta, timezone
from pathlib import Path

PUBLICATION = "https://recsys.substack.com"
ARXIV_API = "https://export.arxiv.org/api/query"
USER_AGENT = "Mozilla/5.0 (compatible; substack-paper-crawler/1.0)"

# arXiv asks for no more than one request every three seconds.
ARXIV_DELAY = 3.0
DOWNLOAD_DELAY = 1.0

# Minimum compact-title similarity for an arXiv hit to count as the same paper.
MATCH_THRESHOLD = 0.90


# --------------------------------------------------------------------------- http


def get(url: str, retries: int = 4, timeout: int = 60) -> bytes:
    """GET a URL with exponential backoff on transient failures."""
    delay = 2.0
    last: Exception | None = None
    for attempt in range(retries):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return resp.read()
        except urllib.error.HTTPError as exc:
            # 404 means the resource genuinely is not there; retrying will not help.
            if exc.code in (400, 403, 404):
                raise
            last = exc
        except Exception as exc:  # noqa: BLE001 - network errors are varied
            last = exc
        if attempt < retries - 1:
            time.sleep(delay)
            delay *= 2
    raise RuntimeError(f"GET failed for {url}: {last}")


def get_json(url: str):
    return json.loads(get(url))


# ----------------------------------------------------------------------- newsletter


@dataclass
class Paper:
    index: int
    title: str
    slug: str
    editorial_title: str = ""
    source: str = ""
    arxiv_id: str = ""
    abs_url: str = ""
    pdf_url: str = ""
    pdf_path: str = ""
    resolved_by: str = ""
    note: str = ""


@dataclass
class Issue:
    post_id: int
    date: str
    slug: str
    title: str
    url: str
    audience: str
    papers: list[Paper] = field(default_factory=list)


def list_issues(cutoff: str) -> list[dict]:
    """Return archive entries published on or after ``cutoff`` (YYYY-MM-DD)."""
    issues, offset = [], 0
    while True:
        batch = get_json(
            f"{PUBLICATION}/api/v1/archive?sort=new&limit=50&offset={offset}"
        )
        if not batch:
            break
        issues.extend(batch)
        if batch[-1]["post_date"][:10] < cutoff:
            break
        offset += 50
    keep = [i for i in issues if i["post_date"][:10] >= cutoff]
    keep.sort(key=lambda i: i["post_date"])
    return keep


def deslug(slug: str) -> str:
    """`8-the-matryoshka-hypencoder` -> `the matryoshka hypencoder`."""
    slug = urllib.parse.unquote(slug)
    return re.sub(r"^\d+-", "", slug).replace("-", " ").strip()


def compact(text: str) -> str:
    """Strip every character that slugification would drop, for robust comparison."""
    return re.sub(r"[^a-z0-9]", "", text.lower())


def strip_tags(fragment: str) -> str:
    return html.unescape(re.sub(r"<[^>]+>", "", fragment)).strip()


def parse_issue(post: dict) -> Issue:
    """Extract the paper list from a post's HTML body.

    The table of contents is present in both paid and free posts. Free posts also
    carry the full body, from which the canonical arXiv link per section is read.
    """
    body = post.get("body_html") or ""
    issue = Issue(
        post_id=post["id"],
        date=post["post_date"][:10],
        slug=post["slug"],
        title=post["title"],
        url=post["canonical_url"],
        audience=post.get("audience", ""),
    )

    # 1. Table of contents: <a href="https://.../i/<post_id>/<paper-slug>">Editorial title</a>, from X
    toc = re.findall(
        rf'<a href="[^"]*?/i/{issue.post_id}/([^"#]+)"[^>]*>(.*?)</a>(.*?)</p>',
        body,
        re.S,
    )
    seen: set[str] = set()
    for slug, editorial, tail in toc:
        if slug in seen:
            continue
        seen.add(slug)
        num = re.match(r"^(\d+)-", slug)
        source = ""
        m = re.search(r"from\s+(.+?)\s*$", strip_tags(tail))
        if m:
            source = m.group(1).rstrip(".")
        issue.papers.append(
            Paper(
                index=int(num.group(1)) if num else len(issue.papers) + 1,
                title=deslug(slug),
                slug=slug,
                editorial_title=strip_tags(editorial),
                source=source,
            )
        )

    # 2. Full body (free posts): <h4>[N] <strong>Real Title</strong></h4> ... 📚 <a href="arxiv">
    sections = re.split(r"<h4>\[(\d+)\]", body)
    by_index = {p.index: p for p in issue.papers}
    for i in range(1, len(sections) - 1, 2):
        num = int(sections[i])
        chunk = sections[i + 1]
        paper = by_index.get(num)
        if paper is None:
            continue
        heading = re.match(r"\s*(.*?)</h4>", chunk, re.S)
        if heading:
            real = strip_tags(heading.group(1))
            if real:
                paper.title = real
        link = re.search(r'href="(https?://arxiv\.org/abs/[^"#?]+)"', chunk)
        if link:
            paper.abs_url = link.group(1)
            paper.arxiv_id = link.group(1).rsplit("/", 1)[-1]
            paper.resolved_by = "newsletter-link"

    issue.papers.sort(key=lambda p: p.index)
    return issue


# ---------------------------------------------------------------------------- arxiv


def arxiv_query(params: str, max_results: int = 8) -> list[tuple[str, str]]:
    """Run an arXiv API query, returning (arxiv_id, title) pairs."""
    url = (
        f"{ARXIV_API}?search_query={urllib.parse.quote(params)}"
        f"&start=0&max_results={max_results}"
    )
    try:
        raw = get(url).decode("utf-8", "replace")
    except Exception as exc:  # noqa: BLE001
        print(f"    arXiv query failed: {exc}", file=sys.stderr)
        return []
    hits = []
    for entry in re.findall(r"<entry>(.*?)</entry>", raw, re.S):
        eid = re.search(r"<id>(.*?)</id>", entry)
        title = re.search(r"<title>(.*?)</title>", entry, re.S)
        if not eid or not title:
            continue
        hits.append(
            (
                eid.group(1).rsplit("/", 1)[-1],
                html.unescape(" ".join(title.group(1).split())),
            )
        )
    return hits


def best_match(target: str, hits: list[tuple[str, str]]) -> tuple[str, str, float]:
    """Pick the hit whose compacted title is closest to ``target``."""
    from difflib import SequenceMatcher

    goal = compact(target)
    best = ("", "", 0.0)
    for arxiv_id, title in hits:
        cand = compact(title)
        # A newsletter title is sometimes the paper's main title without its subtitle.
        ratio = max(
            SequenceMatcher(None, goal, cand).ratio(),
            SequenceMatcher(None, goal, cand[: len(goal)]).ratio() if cand else 0.0,
        )
        if ratio > best[2]:
            best = (arxiv_id, title, ratio)
    return best


# Papers whose newsletter title cannot be reached by an arXiv title search, keyed by
# the section slug. Almost all of these are papers the authors retitled in a later
# arXiv version, so the newsletter records the v1 title while the index serves the v2
# one. Each was confirmed by reading the arXiv abstract and author list.
OVERRIDES = {
    # v1 "...in Industrial Advertising"
    "4-unified-value-alignment-for-generative-recommendation-in-industrial-advertising": (
        "2605.05803",
        "UniVA: Unified Value Alignment for Generative Recommendation in Online Advertising at Tencent",
    ),
    # v1 "...The Next Frontier of Information Retrieval"
    "5-superintelligent-retrieval-agent-the-next-frontier-of-information-retrieval": (
        "2605.06647",
        "Superintelligent Retrieval Agent: The Next Frontier of Agentic Retrieval",
    ),
    # v1 "Test-Time Compute for Dense Retrieval: Agentic Program Generation with Frozen Embedding Models"
    "9-test-time-compute-for-dense-retrieval-agentic-program-generation-with-frozen-embedding-models": (
        "2605.11374",
        "Test-Time Compute for Frozen Embedding Models through Agentic Program Search",
    ),
    # v1 "Long-Term Optimization for Large-Scale Generative Retrieval with Off-Policy
    # REINFORCE"; the VK authors match the newsletter's "from AI VK" attribution
    "10-long-term-optimization-for-large-scale-generative-retrieval-with-off-policy-reinforce": (
        "2607.02818",
        "Session-Level Optimization for Large-Scale Retrieval using REINFORCE with Multi-Step Off-Policy Correction",
    ),
}

# Papers confirmed to have no openly downloadable PDF, keyed by section slug.
NOT_AVAILABLE = {
    "8-tmmsrec-time-interval-aware-multi-modal-sequential-recommender": (
        "not on arXiv; no open PDF found via arXiv or web search (venue-only paper)"
    ),
}


# `and`/`or`/`not` are operators in the arXiv query grammar, and the remaining words
# are too common to narrow a title search; including any of them yields zero hits.
STOPWORDS = {
    "and", "or", "not", "the", "a", "an", "of", "for", "to", "in", "on", "with",
    "by", "is", "are", "as", "at", "from", "that", "this", "it", "its", "be",
    "can", "do", "does", "we", "you", "via", "using", "into", "when", "how",
}


def resolve(paper: Paper) -> None:
    """Fill in ``arxiv_id`` for a paper known only by title."""
    if paper.arxiv_id:
        return
    if paper.slug in NOT_AVAILABLE:
        paper.note = NOT_AVAILABLE[paper.slug]
        paper.resolved_by = "unavailable"
        return
    if paper.slug in OVERRIDES:
        paper.arxiv_id, paper.title = OVERRIDES[paper.slug]
        paper.abs_url = f"https://arxiv.org/abs/{paper.arxiv_id}"
        paper.resolved_by = "override"
        return
    terms, seen = [], set()
    for token in re.findall(r"[a-z0-9]+", paper.title.lower()):
        if len(token) > 1 and token not in STOPWORDS and token not in seen:
            seen.add(token)
            terms.append(token)
    # Slugification destroys punctuation, so a token like `v35` (from `v3.5`) is not
    # in the arXiv index at all. Preferring the longest tokens routes around those.
    longest = sorted(terms, key=len, reverse=True)
    strategies = [
        f'ti:"{paper.title}"',
        " AND ".join(f"ti:{t}" for t in terms),
        " AND ".join(f"ti:{t}" for t in longest[:5]),
        " AND ".join(f"ti:{t}" for t in longest[:3]),
        f'all:"{paper.title}"',
    ]
    strategies = [s for s in strategies if s]
    for i, query in enumerate(strategies):
        hits = arxiv_query(query)
        time.sleep(ARXIV_DELAY)
        if not hits:
            continue
        arxiv_id, title, ratio = best_match(paper.title, hits)
        if ratio >= MATCH_THRESHOLD:
            paper.arxiv_id = arxiv_id
            paper.title = title
            paper.abs_url = f"https://arxiv.org/abs/{arxiv_id}"
            paper.resolved_by = f"arxiv-search:{i}"
            return
        paper.note = f"best arXiv candidate {arxiv_id!r} scored {ratio:.2f}"
    if not paper.arxiv_id:
        paper.note = paper.note or "no arXiv match"


def canonicalize_titles(issues: list[Issue], batch: int = 50) -> int:
    """Replace each resolved paper's title with arXiv's official metadata title.

    Titles arrive from three places -- a newsletter heading, a de-slugified anchor, or
    a search hit -- so without this the filenames are inconsistent. arXiv's `id_list`
    accepts many IDs per call, making this a handful of requests rather than one each.
    """
    papers = [p for i in issues for p in i.papers if p.arxiv_id]
    official: dict[str, str] = {}
    for start in range(0, len(papers), batch):
        chunk = papers[start : start + batch]
        ids = ",".join(p.arxiv_id for p in chunk)
        try:
            raw = get(f"{ARXIV_API}?id_list={ids}&max_results={len(chunk)}").decode(
                "utf-8", "replace"
            )
        except Exception as exc:  # noqa: BLE001
            print(f"  metadata batch failed: {exc}", file=sys.stderr)
            continue
        for entry in re.findall(r"<entry>(.*?)</entry>", raw, re.S):
            eid = re.search(r"<id>(.*?)</id>", entry)
            title = re.search(r"<title>(.*?)</title>", entry, re.S)
            if eid and title:
                key = eid.group(1).rsplit("/", 1)[-1]
                official[key] = html.unescape(" ".join(title.group(1).split()))
        time.sleep(ARXIV_DELAY)

    changed = 0
    for paper in papers:
        name = official.get(paper.arxiv_id) or official.get(paper.arxiv_id + "v1")
        if not name:
            # The API echoes the versioned ID; fall back to a version-insensitive match.
            base = paper.arxiv_id.split("v")[0]
            for key, value in official.items():
                if key.split("v")[0] == base:
                    name = value
                    break
        if name and name != paper.title:
            paper.title = name
            changed += 1
    return changed


# -------------------------------------------------------------------------- fetching


def safe_name(text: str, limit: int = 110) -> str:
    name = re.sub(r"[^\w\s-]", "", text).strip()
    name = re.sub(r"[\s_]+", "_", name)
    return name[:limit].rstrip("_") or "untitled"


def download(paper: Paper, directory: Path) -> bool:
    if not paper.arxiv_id:
        return False
    paper.pdf_url = f"https://arxiv.org/pdf/{paper.arxiv_id}"
    dest = directory / f"{paper.index:02d}_{safe_name(paper.title)}.pdf"
    if dest.exists() and dest.stat().st_size > 1024:
        paper.pdf_path = str(dest)
        return True
    try:
        data = get(paper.pdf_url)
    except Exception as exc:  # noqa: BLE001
        paper.note = f"download failed: {exc}"
        return False
    if not data.startswith(b"%PDF"):
        paper.note = "response was not a PDF"
        return False
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(data)
    paper.pdf_path = str(dest)
    time.sleep(DOWNLOAD_DELAY)
    return True


# ------------------------------------------------------------------------------ main


def write_index(issues: list[Issue], out: Path) -> None:
    total = sum(len(i.papers) for i in issues)
    got = sum(1 for i in issues for p in i.papers if p.pdf_path)
    lines = [
        "# Papers from recsys.substack.com",
        "",
        f"Newsletter: <{PUBLICATION}>  ",
        f"Issues covered: {len(issues)} ({issues[0].date} to {issues[-1].date})  ",
        f"Papers listed: {total} — PDFs downloaded: {got}",
        "",
    ]
    for issue in issues:
        lines += [
            f"## {issue.date} — [{issue.title}]({issue.url})",
            "",
            f"_{issue.audience.replace('_', ' ')}_",
            "",
        ]
        for p in issue.papers:
            link = f"[{p.arxiv_id}]({p.abs_url})" if p.arxiv_id else "_unresolved_"
            pdf = f"`{Path(p.pdf_path).name}`" if p.pdf_path else "—"
            lines.append(f"{p.index}. **{p.title}** — {link} — {pdf}")
            if p.note and not p.pdf_path:
                lines.append(f"   - {p.note}")
        lines.append("")
    (out / "INDEX.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--months", type=int, default=3, help="how far back to crawl")
    ap.add_argument("--since", help="explicit YYYY-MM-DD cutoff (overrides --months)")
    ap.add_argument("--out", default="papers", help="output directory")
    ap.add_argument(
        "--refresh",
        action="store_true",
        help="ignore arXiv IDs cached in an existing manifest and look them all up again",
    )
    ap.add_argument(
        "--free-only",
        action="store_true",
        help="only crawl issues published to everyone, skipping subscriber-only ones",
    )
    args = ap.parse_args()

    cutoff = args.since or (
        datetime.now(timezone.utc) - timedelta(days=31 * args.months)
    ).strftime("%Y-%m-%d")
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)

    print(f"Crawling {PUBLICATION} issues since {cutoff}")
    entries = list_issues(cutoff)
    print(f"Found {len(entries)} issues")
    if args.free_only:
        kept = [e for e in entries if e.get("audience") == "everyone"]
        print(f"  --free-only: keeping {len(kept)}, skipping {len(entries) - len(kept)}")
        entries = kept

    issues: list[Issue] = []
    for entry in entries:
        post = get_json(f"{PUBLICATION}/api/v1/posts/by-id/{entry['id']}")
        post = post.get("post", post)
        post.setdefault("canonical_url", entry["canonical_url"])
        issue = parse_issue(post)
        issues.append(issue)
        print(f"  {issue.date}  {len(issue.papers):2d} papers  ({issue.audience})")

    # Reuse IDs from a previous run so re-crawling costs one arXiv request per new
    # paper rather than one per paper.
    # The cache must carry the resolved title as well as the ID: the title is what
    # names the PDF, so caching the ID alone would make a re-run write a second copy
    # of every paper under its unresolved slug-derived name.
    cache: dict[str, tuple[str, str]] = {}
    old = out / "manifest.json"
    if old.exists() and not args.refresh:
        for issue in json.loads(old.read_text(encoding="utf-8"))["issues"]:
            for paper in issue["papers"]:
                if paper["arxiv_id"]:
                    cache[paper["slug"]] = (paper["arxiv_id"], paper["title"])
        print(f"Reusing {len(cache)} cached arXiv IDs (--refresh to ignore)")

    print("\nResolving arXiv IDs...")
    for issue in issues:
        for paper in issue.papers:
            # The cache is consulted only after the hand-checked tables, so a stale
            # entry from an earlier run cannot shadow a correction made since.
            if (
                not paper.arxiv_id
                and paper.slug in cache
                and paper.slug not in OVERRIDES
                and paper.slug not in NOT_AVAILABLE
            ):
                paper.arxiv_id, paper.title = cache[paper.slug]
                paper.abs_url = f"https://arxiv.org/abs/{paper.arxiv_id}"
                paper.resolved_by = "cache"
            resolve(paper)
            state = paper.arxiv_id or f"UNRESOLVED ({paper.note})"
            print(f"  [{issue.date} #{paper.index}] {state}  {paper.title[:70]}")

    print("\nCanonicalizing titles from arXiv metadata...")
    print(f"  updated {canonicalize_titles(issues)} titles")

    print("\nDownloading PDFs...")
    for issue in issues:
        directory = out / f"{issue.date}_{issue.slug}"
        for paper in issue.papers:
            ok = download(paper, directory)
            print(f"  {'OK ' if ok else 'MISS'} {issue.date} #{paper.index} {paper.title[:60]}")

    manifest = {
        "publication": PUBLICATION,
        "cutoff": cutoff,
        "issues": [
            {**asdict(issue), "papers": [asdict(p) for p in issue.papers]}
            for issue in issues
        ],
    }
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False), encoding="utf-8"
    )
    write_index(issues, out)

    total = sum(len(i.papers) for i in issues)
    got = sum(1 for i in issues for p in i.papers if p.pdf_path)
    print(f"\nDone: {got}/{total} PDFs in {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
