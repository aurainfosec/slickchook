#!/usr/bin/env python3
"""Thin wrapper around Debian's psk-crack binary (bundled in the same
ike-scan package as the ike-scan plugin).

psk-crack's own man page synopsis shows `< psk-parameters-file>` as if the
data can only arrive via shell stdin redirection, but its real C source
(royhills/ike-scan, psk-crack.c) confirms it actually takes that file as a
genuine positional argv argument (`argv[optind]`, read via a plain
fopen() - "-" specifically means stdin, anything else is a real path).
This fleet's plugin schema has no mechanism to hand a container an inline
multi-line blob except as a CLI option value, so this wrapper accepts that
blob as its own --slickchook-psk-params option, writes it verbatim to a
file, and execs the real psk-crack with that file as its positional
argument plus whatever real psk-crack flags were also supplied.
"""
import argparse
import os
import subprocess
import sys
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--slickchook-psk-params", required=True, dest="psk_params")
    args, passthrough = parser.parse_known_args()

    text = args.psk_params
    if not text.endswith("\n"):
        text += "\n"

    fd, path = tempfile.mkstemp(prefix="psk-params-", suffix=".txt", dir="/tmp")
    with os.fdopen(fd, "w") as f:
        f.write(text)

    cmd = ["/usr/bin/psk-crack"] + passthrough + [path]
    result = subprocess.run(cmd)
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
