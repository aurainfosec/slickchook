#!/usr/bin/env python3
"""Translates declared CLI options into the exact stdin sequence CUPP's
`-i` interactive mode expects - CUPP (github.com/Mebus/cupp) has no
non-interactive/flag-based way to build a personalized wordlist at all,
confirmed by reading its own source (cmd/main dispatch only offers -i
interactive, -w improve-existing-file, -l download-wordlist, -a
alecto-db, none of which take profile fields as flags).

The exact prompt order below (including the "Hyperspeed Print?" prompt,
which is easy to miss - it lives in a helper function called at the very
end, not in the interactive() function itself) was confirmed by running
the real script end-to-end with a synthetic stdin sequence, not assumed
from reading the source alone.
"""
import argparse
import os
import re
import subprocess
import sys

BIRTHDATE_RE = re.compile(r"^\d{8}$")


def validate_birthdate(value, label):
    if value and not BIRTHDATE_RE.match(value):
        sys.stderr.write(
            f"error: {label} must be exactly 8 digits (DDMMYYYY) or omitted, got {value!r}\n"
        )
        sys.exit(2)
    return value


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--name", required=True)
    parser.add_argument("--surname", default="")
    parser.add_argument("--nickname", default="")
    parser.add_argument("--birthdate", default="")
    parser.add_argument("--partner-name", default="")
    parser.add_argument("--partner-nickname", default="")
    parser.add_argument("--partner-birthdate", default="")
    parser.add_argument("--child-name", default="")
    parser.add_argument("--child-nickname", default="")
    parser.add_argument("--child-birthdate", default="")
    parser.add_argument("--pet-name", default="")
    parser.add_argument("--company", default="")
    parser.add_argument("--keywords", default="")
    parser.add_argument("--special-chars", action="store_true")
    parser.add_argument("--random-numbers", action="store_true")
    parser.add_argument("--leet-mode", action="store_true")
    args = parser.parse_args()

    validate_birthdate(args.birthdate, "birthdate")
    validate_birthdate(args.partner_birthdate, "partner-birthdate")
    validate_birthdate(args.child_birthdate, "child-birthdate")

    lines = [
        args.name,
        args.surname,
        args.nickname,
        args.birthdate,
        args.partner_name,
        args.partner_nickname,
        args.partner_birthdate,
        args.child_name,
        args.child_nickname,
        args.child_birthdate,
        args.pet_name,
        args.company,
        "y" if args.keywords else "n",
    ]
    if args.keywords:
        lines.append(args.keywords)
    lines += [
        "y" if args.special_chars else "n",
        "y" if args.random_numbers else "n",
        "y" if args.leet_mode else "n",
        "n",  # Hyperspeed Print? - never useful in a captured, non-interactive run
    ]
    stdin_data = "\n".join(lines) + "\n"

    # cupp.py writes its output as "<name>.txt" in the current directory,
    # where <name> is the same first-name value it was given (lowercased,
    # stripped) - running from /artifacts directly means the file lands
    # exactly where it needs to for read_artifact/list_run_artifacts to
    # find it, without needing to know the exact filename in advance or
    # rename anything afterward.
    os.chdir("/artifacts")
    result = subprocess.run(
        [sys.executable, "/opt/cupp/cupp.py", "-i", "-q"],
        input=stdin_data,
        text=True,
    )
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
