#!/usr/bin/env python3
"""Bounds and dispatches a real pagodo.py (github.com/opsdisk/pagodo) run.

pagodo has no built-in way to cap how many dorks a single run processes -
its own bundled GHDB corpus is 7,944 dorks total, and even its smallest
single category (121 dorks) would take roughly 97 minutes at its own
default delay, well past what's reasonable for one bounded container run.
max_dorks (a new option this wrapper adds, not present upstream) slices
the requested category's dork file down to a small, deliberate batch - a
human runs this repeatedly, choosing to cover more of a category each
time, rather than the container attempting an entire category in one
shot.

Dork files are discovered dynamically from wherever the image's build-time
`ghdb_scraper.py -s -i` scrape actually wrote them, rather than assuming a
hardcoded path - if a category name doesn't match, the real list of what
was actually produced is reported back instead of silently failing.
"""
import argparse
import glob
import os
import subprocess
import sys

DORKS_ROOT = "/opt/pagodo"


def find_dork_files():
    return sorted(glob.glob(os.path.join(DORKS_ROOT, "**", "*.dorks"), recursive=True))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--category", required=True)
    parser.add_argument("--max-dorks", type=int, default=15, dest="max_dorks")
    parser.add_argument("--min-delay", type=int, default=37, dest="min_delay")
    parser.add_argument("--max-delay", type=int, default=60, dest="max_delay")
    parser.add_argument("--max-urls-per-dork", type=int, default=100, dest="max_urls_per_dork")
    args = parser.parse_args()

    dork_files = find_dork_files()
    matches = [f for f in dork_files if os.path.splitext(os.path.basename(f))[0] == args.category]
    if not matches:
        available = sorted({os.path.splitext(os.path.basename(f))[0] for f in dork_files})
        sys.stderr.write(
            f"error: unknown category {args.category!r}. Available "
            f"categories baked into this image: {', '.join(available)}\n"
        )
        sys.exit(2)
    source_file = matches[0]

    with open(source_file) as f:
        lines = [line for line in f if line.strip()]
    bounded = lines[: args.max_dorks]
    if not bounded:
        sys.stderr.write(f"error: {source_file} contains no dorks\n")
        sys.exit(2)

    bounded_path = "/tmp/bounded.dorks"
    with open(bounded_path, "w") as f:
        f.writelines(bounded)

    sys.stderr.write(
        f"Running {len(bounded)} of {len(lines)} dorks in category "
        f"{args.category!r} (max_dorks={args.max_dorks}).\n"
    )

    cmd = [
        sys.executable,
        "/opt/pagodo/pagodo.py",
        "-g", bounded_path,
        "-d", args.target,
        "-i", str(args.min_delay),
        "-x", str(args.max_delay),
        "-m", str(args.max_urls_per_dork),
        "-o", "/artifacts/results.json",
        "-s", "/artifacts/results.txt",
        "-z", "/artifacts/pagodo.log",
    ]
    result = subprocess.run(cmd, cwd="/opt/pagodo")
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
