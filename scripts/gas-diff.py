#!/usr/bin/env python3
"""Compare two gas snapshot files and output a markdown table of changes."""

import sys


def parse_snapshot(path):
    result = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.rsplit(",", 4)
            name = parts[0].strip().rstrip(",")
            vals = [int(p.strip()) for p in parts[1:]]
            result[name] = vals
    return result


def fmt(old, new):
    diff = new - old
    pct = (diff / old * 100) if old != 0 else 0
    sign = "+" if diff > 0 else ""
    return f"{sign}{diff} ({sign}{pct:.1f}%)"


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <base-snapshot> <head-snapshot>", file=sys.stderr)
        sys.exit(1)

    base = parse_snapshot(sys.argv[1])
    head = parse_snapshot(sys.argv[2])

    all_names = sorted(set(base.keys()) | set(head.keys()))
    new_tests = []
    removed_tests = []
    changed = []
    unchanged = []

    for name in all_names:
        if name not in base:
            new_tests.append((name, head[name]))
        elif name not in head:
            removed_tests.append((name, base[name]))
        elif base[name] != head[name]:
            changed.append((name, base[name], head[name]))
        else:
            unchanged.append(name)

    if not changed and not new_tests and not removed_tests:
        print("No gas changes detected.")
        return

    # Sort by largest percentage decrease in O3 (index 3)
    changed.sort(
        key=lambda x: (x[2][3] - x[1][3]) / x[1][3] if x[1][3] != 0 else 0
    )

    print("## Gas Snapshot Diff\n")

    if new_tests:
        print(f"### New tests ({len(new_tests)})\n")
        print("| Test | O0 | O1 | O2 | O3 |")
        print("|------|---:|---:|---:|---:|")
        for name, vals in new_tests:
            print(f"| `{name}` | {vals[0]} | {vals[1]} | {vals[2]} | {vals[3]} |")
        print()

    if removed_tests:
        print(f"### Removed tests ({len(removed_tests)})\n")
        print("| Test | O0 | O1 | O2 | O3 |")
        print("|------|---:|---:|---:|---:|")
        for name, vals in removed_tests:
            print(f"| `{name}` | {vals[0]} | {vals[1]} | {vals[2]} | {vals[3]} |")
        print()

    print(
        f"### Changed ({len(changed)}) | Unchanged ({len(unchanged)})\n"
    )
    print("| Test | O0 | O1 | O2 | O3 |")
    print("|------|---:|---:|---:|---:|")

    total_base = [0, 0, 0, 0]
    total_head = [0, 0, 0, 0]

    for name, old, new in changed:
        cols = []
        for i in range(4):
            cols.append(fmt(old[i], new[i]))
            total_base[i] += old[i]
            total_head[i] += new[i]
        print(f"| `{name}` | {cols[0]} | {cols[1]} | {cols[2]} | {cols[3]} |")

    totals = [fmt(total_base[i], total_head[i]) for i in range(4)]
    print(
        f"| **TOTAL** | **{totals[0]}** | **{totals[1]}** | **{totals[2]}** | **{totals[3]}** |"
    )

    regressions = [(n, o, nw) for n, o, nw in changed if nw[3] > o[3]]
    if regressions:
        print(f"\n### Regressions at O3 ({len(regressions)})\n")
        print("| Test | O3 |")
        print("|------|---:|")
        for name, old, new in regressions:
            print(f"| `{name}` | {fmt(old[3], new[3])} |")


if __name__ == "__main__":
    main()
