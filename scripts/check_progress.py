#!/usr/bin/env python3
"""Validate milestone structure, dependencies and evidence; print progress."""
import json
import re
from pathlib import Path
from collections import Counter

ROOT = Path(__file__).resolve().parents[1]
STATES = {"planned", "in_progress", "blocked", "implemented", "done"}

def main():
    data = json.loads((ROOT / "docs/progress.json").read_text(encoding="utf-8"))
    rows = data["milestones"]
    errors = []
    by_id = {m["id"]: m for m in rows}
    if data.get("schema_version") != 1 or len(by_id) != len(rows) or not rows:
        errors.append("invalid schema or duplicate/empty milestones")
    for m in rows:
        ident, state = m["id"], m["status"]
        if state not in STATES or not m.get("acceptance"):
            errors.append(f"{ident}: invalid status or missing acceptance")
        for dep in m["depends_on"]:
            if dep not in by_id or dep == ident:
                errors.append(f"{ident}: invalid dependency {dep}")
            elif state == "done" and by_id[dep]["status"] != "done":
                errors.append(f"{ident}: dependency {dep} not done")
        if state == "blocked" and not m.get("blocker"):
            errors.append(f"{ident}: missing blocker")
        if state == "done":
            ev = m.get("evidence", [])
            if not ev or not all(e.get("result") == "pass" and all(e.get(k) for k in ("commit", "environment", "artifact")) for e in ev):
                errors.append(f"{ident}: missing passing evidence")
    visiting, visited = set(), set()
    def visit(ident):
        if ident in visiting:
            errors.append(f"dependency cycle: {ident}")
            return
        if ident in visited or ident not in by_id:
            return
        visiting.add(ident)
        for dep in by_id[ident]["depends_on"]:
            visit(dep)
        visiting.remove(ident)
        visited.add(ident)
    for ident in by_id:
        visit(ident)
    md = (ROOT / "docs/PROGRESS.md").read_text(encoding="utf-8")
    actual = dict(re.findall(r"^\| (M\d+) \| .*? \| (\w+) \|$", md, re.M))
    if actual != {m["id"]: m["status"] for m in rows}:
        errors.append("PROGRESS.md and progress.json disagree")
    counts = Counter(m["status"] for m in rows)
    print(f"Milestones done: {counts['done']}/{len(rows)} ({100*counts['done']/len(rows):.1f}%)" if rows else "No milestones")
    for state in sorted(STATES):
        print(f"{state}: {counts[state]}")
    for m in rows:
        if m["status"] == "planned" and all(by_id.get(d, {}).get("status") == "done" for d in m["depends_on"]):
            print(f"Ready: {m['id']} {m['title']}")
    for err in errors:
        print(f"ERROR: {err}")
    return bool(errors)

if __name__ == "__main__":
    raise SystemExit(main())
