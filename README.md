# codex-test-2

This repository contains a sample Python web application demonstrating how to use OpenAI's ChatGPT API to get electric guitar practice tips. See the `guitar_gpt` directory for details. The interface displays ASCII guitar tabs and can now play them back through your browser.

## Search engine research

* [`posting_bench`](posting_bench/README.md): compressed posting lists and their intersection and union algorithms in Rust, a benchmark testbed, and three new designs (Pyramid, Loom, Entropic). Reports are in `posting_bench/report/`.
* [`strata`](strata/README.md): Strata, an embedded single-file search database with SQL that combines posting lists, a columnar forward index and vector ANN indexes, with hybrid ranking and a cost-based planner. Evaluated against SQLite (FTS5) and Tantivy on four BEIR collections; the report is built into `strata/report/strata.html`.
* [`engram`](engram/README.md): Engram, automatic long-term context for Claude Code built on Strata. Hooks store every prompt, tool call, answer and compaction summary. A per-project daemon decides when to inject relevant history, and MCP tools let the model search it. It is evaluated on LongMemEval, LoCoMo, this repository's own 26-hour session, and live Claude Code runs.
