#!/usr/bin/env python3
"""Thin HTTP wrapper around the Brave Search API's web search endpoint
(api.search.brave.com/res/v1/web/search), used here for dork-style passive
discovery of exposed files/content on one authorized target domain.

No official CLI/SDK exists for this - a single plain HTTP GET with one
header - so this is a from-scratch stdlib-only client, matching the
`dehashed` plugin's pattern.

Unlike Google, only site:, filetype:, quoted phrases, and -exclusion are
confirmed (from Brave's own query-syntax docs) to actually work here -
intitle:/inurl:/intext:, which most classic Google Hacking Database dorks
rely on, are not documented as supported and are assumed not to work.
Don't add preset "dork" queries built from unconfirmed operators - only
ship what's actually been verified against the real docs.

Scope safety is enforced structurally, not just documented: target is a
required, separate field, and this script always prepends `site:<target>`
to the dork text itself before sending the query - there is no way to
invoke this plugin with an unscoped, whole-internet dork query.
"""
import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

API_URL = "https://api.search.brave.com/res/v1/web/search"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--dork", required=True)
    parser.add_argument("--freshness")
    parser.add_argument("--country")
    parser.add_argument("--safesearch", choices=["off", "moderate", "strict"])
    parser.add_argument("--count", type=int)
    parser.add_argument("--offset", type=int)
    parser.add_argument("--extra-snippets", action="store_true", dest="extra_snippets")
    args = parser.parse_args()

    api_key = os.environ.get("BRAVE_API_KEY")
    if not api_key:
        sys.stderr.write(
            'error: BRAVE_API_KEY must be configured on the slickchook '
            'service - see RUNNER.md\'s "Configuring plugin API keys and '
            'secrets"\n'
        )
        sys.exit(2)

    query = f"site:{args.target} {args.dork}"
    params = {"q": query}
    if args.freshness:
        params["freshness"] = args.freshness
    if args.country:
        params["country"] = args.country
    if args.safesearch:
        params["safesearch"] = args.safesearch
    if args.count is not None:
        params["count"] = args.count
    if args.offset is not None:
        params["offset"] = args.offset
    if args.extra_snippets:
        params["extra_snippets"] = "true"

    url = f"{API_URL}?{urllib.parse.urlencode(params)}"
    request = urllib.request.Request(
        url,
        method="GET",
        headers={
            "Accept": "application/json",
            "X-Subscription-Token": api_key,
        },
    )

    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read()
    except urllib.error.HTTPError as e:
        raw = e.read()
        sys.stderr.write(f"error: Brave Search API returned HTTP {e.code}\n")
        try:
            sys.stderr.write(json.dumps(json.loads(raw), indent=2) + "\n")
        except ValueError:
            sys.stderr.write(raw.decode("utf-8", errors="replace") + "\n")
        sys.exit(1)
    except urllib.error.URLError as e:
        sys.stderr.write(f"error: could not reach Brave Search API: {e.reason}\n")
        sys.exit(1)

    try:
        parsed = json.loads(raw)
    except ValueError:
        sys.stdout.buffer.write(raw)
        return

    print(json.dumps(parsed, indent=2))


if __name__ == "__main__":
    main()
