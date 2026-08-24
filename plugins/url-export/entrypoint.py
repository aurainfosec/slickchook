#!/usr/bin/env python3
"""Extracts, normalizes, and deduplicates URLs/domains out of an arbitrary
blob of already-gathered recon text (raw stdout/artifact content pasted in
by the caller, however messy - plain lines, JSON arrays, mixed punctuation)
into one clean, plain-text URL list - the portable, universally-importable
export format almost every other tool (including Burp Suite) can ingest
one way or another.

Deliberately does not try to parse any particular plugin's own output
format - every plugin here has a different shape (a plain URL list, a
JSON array, hostnames with no scheme, etc), and hardcoding per-plugin
parsing here would duplicate knowledge the calling AI already has about
each plugin's own declared output. Instead this is a generic
extract-anything-URL-shaped-and-clean-it-up pass over whatever text is
handed to it.
"""
import argparse
import json
import re
import sys
from urllib.parse import urlparse

URL_RE = re.compile(r"https?://[^\s\"'<>\\)\]]+")
BARE_HOST_RE = re.compile(
    r"^[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?"
    r"(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$"
)
TRAILING_PUNCT = '.,;:)]}"\'`'


def normalize_url(raw):
    url = raw.rstrip(TRAILING_PUNCT)
    try:
        parsed = urlparse(url)
    except ValueError:
        return None
    if not parsed.scheme or not parsed.netloc:
        return None
    path = parsed.path or "/"
    rebuilt = f"{parsed.scheme}://{parsed.netloc}{path}"
    if parsed.query:
        rebuilt += f"?{parsed.query}"
    return rebuilt


def host_in_scope(host, allowed_domains):
    host = host.split(":")[0].lower()
    for domain in allowed_domains:
        domain = domain.lower().lstrip(".")
        if host == domain or host.endswith("." + domain):
            return True
    return False


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--text", required=True)
    parser.add_argument("--default-scheme", default="")
    parser.add_argument("--include-only-domain", default="")
    args = parser.parse_args()

    allowed_domains = [d.strip() for d in args.include_only_domain.split(",") if d.strip()]

    found = set()
    for match in URL_RE.finditer(args.text):
        normalized = normalize_url(match.group(0))
        if normalized:
            found.add(normalized)

    if args.default_scheme:
        for raw_line in args.text.splitlines():
            token = raw_line.strip().strip(TRAILING_PUNCT + " ")
            if token and BARE_HOST_RE.match(token):
                found.add(f"{args.default_scheme}://{token}/")

    if allowed_domains:
        found = {u for u in found if host_in_scope(urlparse(u).netloc, allowed_domains)}

    result = sorted(found)

    with open("/artifacts/urls.txt", "w") as f:
        f.write("\n".join(result) + ("\n" if result else ""))
    with open("/artifacts/urls.json", "w") as f:
        json.dump(result, f, indent=2)

    print(f"{len(result)} unique URL(s) extracted")
    for u in result:
        print(u)


if __name__ == "__main__":
    main()
