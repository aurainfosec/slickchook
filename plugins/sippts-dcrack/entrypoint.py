#!/usr/bin/env python3
"""Thin wrapper around sippts's real `dcrack` subcommand.

dcrack reads its SIP Digest hashes from a real file via its own -f flag
(confirmed from source, sipdigestcrack.py: a plain `open(self.file, "r")`,
one entry per line, in the classic SipCrack format: ipsrc"ipdst"username
"realm"method"uri"nonce"cnonce"nc"qop"algorithm"response). This fleet's
plugin schema has no mechanism to hand a container an inline multi-line
blob except as a CLI option value, so this wrapper accepts that blob as
its own --slickchook-hashes option, writes it verbatim to a file, and
execs the real `sippts dcrack` with -f pointing at that file, plus
whatever real dcrack flags were also supplied.
"""
import argparse
import os
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--slickchook-hashes", required=True, dest="hashes")
    args, passthrough = parser.parse_known_args()

    text = args.hashes
    if not text.endswith("\n"):
        text += "\n"

    fd, path = tempfile.mkstemp(prefix="sip-hashes-", suffix=".txt", dir="/tmp")
    with os.fdopen(fd, "w") as f:
        f.write(text)

    cmd = ["sippts", "dcrack", "-f", path] + passthrough
    result = subprocess.run(cmd)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
