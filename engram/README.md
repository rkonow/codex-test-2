# Engram

Automatic long-term context for Claude Code, stored in [Strata](../strata).

Engram records everything a coding session does into one Strata file per
project: prompts, answers, tool calls and results, errors, and compaction
summaries. When it helps, Engram brings the right pieces back on its own,
and it gives the model tools to search its history. Nothing depends on the
model remembering to save. The design is in [DESIGN.md](DESIGN.md).

```
 Claude Code ──hooks──▶ engram hook <event> ─┐   (~10 ms per hook)
     └──MCP (stdio)──▶ engram mcp ───────────┤   unix socket
                                              ▼
                         engram serve: one daemon per project, started on demand
                         capture → embed → Strata (BM25 + vectors + filters)
                         recall  → live-window exclusion → priors → gate → pack
```

## What happens automatically

| when | Engram |
|---|---|
| session start, resume, clear, compaction | injects a briefing: stated preferences, decisions, open tasks, where the last session stopped |
| every prompt | stores it; searches older history (never what is still in context); injects the best records when the evidence is strong (up to 4,000 characters), two records when it is weak (1,200), nothing otherwise |
| before `Edit`/`Write` of a file | injects up to two earlier errors or decisions about that file, once per context window |
| every tool call | stores the call and its result (errors flagged, long output trimmed to head and tail, secrets redacted) |
| end of a turn | stores the answer, the commentary written between tool calls, and an episode card (prompt, files changed, commands, errors, outcome) |
| compaction | stores the summary by section; the live window restarts |

The model also gets five MCP tools:
- `memory_search`: hybrid search;
- `memory_timeline`: what surrounded a record;
- `memory_sql`: read-only SQL over the `memory` table;
- `memory_remember` and `memory_forget`.

## Install

```sh
cargo build --release                      # in engram/
target/release/engram fetch-model          # potion-retrieval-32M (static embedder, 129 MB)
target/release/engram install --project /path/to/your/project
```

`install` writes four things:
- the hooks, into `.claude/settings.local.json` (personal and git-ignored; `--shared` writes `.claude/settings.json` instead);
- the MCP server, into `.mcp.json`;
- its approval, through `enabledMcpjsonServers`;
- a permission rule for the memory tools.

The store lives in `~/.local/share/engram/projects/<project>/`, outside the repository.

Optional: `cargo build --release --features bert` and `engram fetch-model bert`
enable BGE-small vectors in the daemon (`"embedder": "bert"` in the store's
`config.json`). They are better for paraphrased questions, but each record takes about 50 ms to embed on a CPU.

## Use by hand

```sh
engram search "why did the planner pick post-filtering"
engram sql "SELECT ts, text FROM memory WHERE kind = 'error' AND MATCH(files, 'exec') ORDER BY ts DESC LIMIT 5"
engram remember decision "Prices are integer cents"
engram ingest-transcript ~/.claude/projects/<project>/<session>.jsonl   # back-fill old sessions
engram stats | briefing | stop
```

`ENGRAM_DISABLE=1` turns the hooks off. `ENGRAM_HOME` and `ENGRAM_MODELS`
relocate the store and the models.

## Evaluation

```sh
python3 scripts/download_memory_benchmarks.py && python3 scripts/convert_memory_benchmarks.py
target/release/engram-bench retrieval --data /home/user/data/memory/longmemeval_s.jsonl --embedder static
target/release/engram-bench retrieval --data /home/user/data/memory/locomo.jsonl --embedder static
target/release/engram-bench probes --transcript <session.jsonl> --probes probes/lost_probes.json --points 1952:4632,4633:5392
target/release/engram-bench scale --data /home/user/data/memory/longmemeval_s.jsonl --sizes 10000,100000,1000000
python3 scripts/fit_gate.py results/retrieval/longmemeval_s_static_round.rows.jsonl results/retrieval/locomo_static_round.rows.jsonl
```

Results are in `results/`, and the report covers them.
