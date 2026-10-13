#!/usr/bin/env python3
"""Regrade long-session A/B runs from their saved patches.

Each run's patch (results/long/<scenario>/patches/<lane>/<task>_<rep>.patch)
is a diff against the lane's repository at the end of its history, which
long_session_ab.py keeps at <work>/<scenario>/<lane>/<scenario>.snap. This
rebuilds each run's repository from that snapshot plus the patch, grades it
again with the current graders, and rewrites runs.jsonl. A regraded row keeps
its first grade as score_v1 / checks_v1.

  python3 scripts/regrade.py kestrel --work /home/user/long            # runs whose grader crashed
  python3 scripts/regrade.py kestrel --work /home/user/long --all      # every run
"""

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ab_eval as ab  # noqa: E402

RESULTS = os.path.join(ab.ROOT, "results", "long")


def crashed(row):
    return isinstance(row.get("checks"), dict) and "error" in row["checks"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario")
    ap.add_argument("--work", default="/home/user/long")
    ap.add_argument("--all", action="store_true", help="regrade every run, not only those whose grader crashed")
    args = ap.parse_args()
    sc = ab.load_scenario(args.scenario)
    grade = {t["id"]: t["grade"] for t in sc["tasks"]}
    res = os.path.join(RESULTS, sc["name"])
    path = os.path.join(res, "runs.jsonl")
    rows = [json.loads(l) for l in open(path) if l.strip()]
    changed = 0
    for row in rows:
        if not (args.all or crashed(row)):
            continue
        lane = row["lane"].replace("+", "_")
        snap = os.path.join(args.work, sc["name"], lane, sc["name"] + ".snap")
        patch = os.path.join(res, "patches", lane, f"{row['task']}_{row['repeat']}.patch")
        if not os.path.isdir(snap):
            sys.exit(f"missing {snap}: the lane's history snapshot is needed to rebuild the run")
        with tempfile.TemporaryDirectory() as tmp:
            repo = os.path.join(tmp, sc["name"])
            shutil.copytree(snap, repo, symlinks=True)
            if os.path.getsize(patch):
                p = subprocess.run(["git", "apply", "--whitespace=nowarn", patch], cwd=repo, capture_output=True, text=True)
                if p.returncode:
                    p = subprocess.run(["patch", "-p1", "-s", "-i", patch], cwd=repo, capture_output=True, text=True)
                if p.returncode:
                    print(f"{row['lane']} {row['task']}_{row['repeat']}: patch did not apply: {p.stderr.strip()[:200]}")
                    continue
            score, checks = grade[row["task"]](repo, row.get("answer") or "")
        if "score_v1" not in row:
            row["score_v1"], row["checks_v1"] = row["score"], row["checks"]
        print(f"{row['lane']:16} {row['task']:20} {row['repeat']}  {row['score']:.2f} -> {score:.2f}  {checks}")
        row["score"], row["solved"], row["checks"] = score, score >= 1.0, checks
        changed += 1
    with open(path, "w") as f:
        f.writelines(json.dumps(r) + "\n" for r in rows)
    print(f"regraded {changed} runs; rerun scripts/showcase_stats.py for the tables")


if __name__ == "__main__":
    main()
