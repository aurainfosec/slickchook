#!/usr/bin/env python3
"""Thin HTTP wrapper around Dehashed's v2 search API (api.dehashed.com/v2/search).

No official CLI exists for this - confirmed by checking the two
well-established unofficial wrappers on GitHub (hmaverickadams/DeHashed-API-Tool,
l4rm4nd/DehashedDumper), both of which are themselves thin `requests`-based
HTTP clients around this exact endpoint, not a published tool worth wrapping
directly. Uses only the standard library to avoid an extra dependency for
what is a single JSON POST request.

Deliberately makes exactly one API call per invocation - no automatic
pagination. Each call costs a real credit regardless of result count, and
`DeHashed-API-Tool`'s own recursive_search() already prompts a human before
paginating past 25 calls - this plugin goes further and never paginates on
its own at all, always returning the exact `balance` Dehashed reports back
so the caller can see precisely what a call cost.
"""
import argparse
import json
import os
import sys
import urllib.error
import urllib.request

API_URL = "https://api.dehashed.com/v2/search"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--query", required=True)
    parser.add_argument("--page", type=int, default=1)
    parser.add_argument("--size", type=int, default=50)
    parser.add_argument("--wildcard", action="store_true")
    parser.add_argument("--regex", action="store_true")
    parser.add_argument("--de-dupe", action="store_true", dest="de_dupe")
    args = parser.parse_args()

    if args.wildcard and args.regex:
        sys.stderr.write("error: wildcard and regex are mutually exclusive\n")
        sys.exit(2)

    api_key = os.environ.get("DEHASHED_API_KEY")
    if not api_key:
        sys.stderr.write(
            'error: DEHASHED_API_KEY must be configured on the slickchook '
            'service - see RUNNER.md\'s "Configuring plugin API keys and '
            'secrets"\n'
        )
        sys.exit(2)

    body = {
        "query": args.query,
        "page": args.page,
        "size": args.size,
        "wildcard": args.wildcard,
        "regex": args.regex,
        "de_dupe": args.de_dupe,
    }
    request = urllib.request.Request(
        API_URL,
        data=json.dumps(body).encode("utf-8"),
        method="POST",
        headers={
            "Content-Type": "application/json",
            "Accept": "application/json",
            "Dehashed-Api-Key": api_key,
        },
    )

    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read()
    except urllib.error.HTTPError as e:
        raw = e.read()
        sys.stderr.write(f"error: Dehashed API returned HTTP {e.code}\n")
        try:
            sys.stderr.write(json.dumps(json.loads(raw), indent=2) + "\n")
        except ValueError:
            sys.stderr.write(raw.decode("utf-8", errors="replace") + "\n")
        sys.exit(1)
    except urllib.error.URLError as e:
        sys.stderr.write(f"error: could not reach Dehashed API: {e.reason}\n")
        sys.exit(1)

    try:
        parsed = json.loads(raw)
    except ValueError:
        sys.stdout.buffer.write(raw)
        return

    print(json.dumps(parsed, indent=2))


if __name__ == "__main__":
    main()
