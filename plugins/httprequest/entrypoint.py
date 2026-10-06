#!/usr/bin/env python3
"""Thin wrapper around curl that turns a newline-separated block of
"Name: Value" header lines into repeated -H flags.

This fleet's plugin schema allows exactly one value per declared option,
with no way to repeat a flag multiple times from a single call (the same
limitation already documented on ike-scan's vendor/trans options) - but a
real HTTP request commonly needs more than one custom header at once
(e.g. Content-Type alongside a custom API key or Authorization header).
This wrapper accepts all of them as a single --slickchook-headers blob,
one "Name: Value" pair per line, and execs the real curl with one -H per
line plus whatever real curl flags were also supplied.
"""
import argparse
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--slickchook-headers", default="", dest="headers")
    args, passthrough = parser.parse_known_args()

    header_args = []
    for line in args.headers.splitlines():
        line = line.strip()
        if not line:
            continue
        header_args += ["-H", line]

    cmd = ["curl"] + header_args + passthrough
    result = subprocess.run(cmd)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
