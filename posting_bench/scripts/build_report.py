"""Assemble report/report.html from the template, the narrative fragments and
results/*.json. Usage: python3 scripts/build_report.py"""
import datetime
import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parent.parent
results = root / "results"
report = root / "report"


def load(name):
    p = results / f"{name}.json"
    return json.loads(p.read_text()) if p.exists() else None


data = {
    "clustered": load("clustered"),
    "uniform": load("uniform"),
    "wikitext": load("wikitext"),
    "kernels": load("kernels"),
    "terms": {k: load(f"terms_{k}") for k in range(2, 7) if load(f"terms_{k}")},
    "generated": datetime.date.today().isoformat(),
}
try:
    data["rustc"] = subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.split()[1]
except (OSError, IndexError):
    pass

html = (report / "template.html").read_text()
html = html.replace("/*__DATA__*/null", json.dumps(data, separators=(",", ":")))
html = html.replace("/*__SCRIPT__*/", (report / "report.js").read_text())
for marker, fragment in [("<!--RESULTS-->", "results.html"), ("<!--TAKEAWAYS-->", "takeaways.html")]:
    f = report / fragment
    html = html.replace(marker, f.read_text() if f.exists() else "")
(report / "report.html").write_text(html)
print(f"wrote {report / 'report.html'} ({len(html) / 1024:.0f} KiB)")
