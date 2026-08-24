#!/usr/bin/env python3
"""Discovers, fetches, reconstructs, and analyzes JavaScript source maps for
one authorized target - a page URL, a direct .js URL, or a direct .map URL.

Self-contained rather than wrapping a third-party tool: the source map
format itself (version/sources/sourcesContent/names/mappings, per the
Source Map spec) is simple, stable JSON - and building the source-path
reconstruction directly means its path-traversal safety can actually be
verified here, rather than trusted from an unresearched external tool. A
source map's `sources` array is attacker-influenced data (it comes from
whatever the target serves) - each entry is defused before being used as a
filesystem path: any URI scheme prefix (webpack://, ng://, etc. - these are
devtools-only synthetic schemes, not real paths) is stripped, then every
`..`/empty segment is dropped and the result is re-joined as a strictly
relative path, with a final canonicalize-and-containment check before any
write - defense in depth, not just the string-cleaning step alone.

Reconstructed source (where sourcesContent was actually present) is
written under /input/sources/... - this run's own input-staging directory
- specifically so the MCP client can chain this run's own run_id as
input_run_id into a follow-up secret-scanning plugin for a deeper pass,
without this plugin needing to invoke that itself (each plugin run is one
container call - chaining across plugins is the client's job, not a
single container's).

A lightweight built-in scan for common sensitive patterns (API keys, JWTs,
private key headers, internal-looking hostnames, absolute local
filesystem paths revealing a real username/machine) runs directly against
whatever text/paths were found, so this plugin is useful standalone even
before any follow-up chaining.
"""
import argparse
import base64
import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
from html.parser import HTMLParser

SOURCEMAP_COMMENT_RE = re.compile(
    r"//[#@]\s*sourceMappingURL\s*=\s*(\S+)\s*$", re.MULTILINE
)

SENSITIVE_PATTERNS = [
    ("aws_access_key_id", re.compile(r"AKIA[0-9A-Z]{16}")),
    ("google_api_key", re.compile(r"AIzaSy[0-9A-Za-z_-]{33}")),
    ("slack_token", re.compile(r"xox[baprs]-[0-9A-Za-z-]{10,}")),
    ("jwt", re.compile(r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+")),
    ("private_key_header", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    (
        "generic_credential_assignment",
        re.compile(
            r"(?i)\b(api[_-]?key|secret|password|passwd|token|access[_-]?key)\b\s*[:=]\s*['\"][^'\"\s]{6,}['\"]"
        ),
    ),
    (
        "internal_host",
        re.compile(
            r"(?i)\b(localhost|127\.0\.0\.1|10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}|[a-z0-9-]+\.internal)\b"
        ),
    ),
]

LOCAL_PATH_PATTERNS = [
    ("developer_username_path", re.compile(r"(?:/Users/|/home/|C:\\Users\\)[^/\\\s'\"]+")),
]


class ScriptTagParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.srcs = []

    def handle_starttag(self, tag, attrs):
        if tag.lower() == "script":
            for name, value in attrs:
                if name.lower() == "src" and value:
                    self.srcs.append(value)


def fetch(url, timeout):
    request = urllib.request.Request(url, headers={"User-Agent": "slickchook-sourcemap-scanner"})
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return response.read()


def find_sourcemap_url(js_text):
    matches = SOURCEMAP_COMMENT_RE.findall(js_text)
    return matches[-1] if matches else None


def load_sourcemap(map_ref, base_url, timeout):
    """Returns (parsed_json, resolved_map_url_or_data_uri) or (None, None)."""
    if map_ref.startswith("data:"):
        try:
            header, encoded = map_ref.split(",", 1)
        except ValueError:
            return None, None
        try:
            if "base64" in header:
                raw = base64.b64decode(encoded)
            else:
                raw = urllib.parse.unquote_to_bytes(encoded)
            return json.loads(raw), "<inline data URI>"
        except (ValueError, json.JSONDecodeError):
            return None, None

    resolved = urllib.parse.urljoin(base_url, map_ref)
    try:
        raw = fetch(resolved, timeout)
        return json.loads(raw), resolved
    except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, ValueError):
        return None, None


def sanitize_source_path(raw_path):
    """Turns an attacker-influenced `sources` entry into a safe relative
    path: strips any URI-scheme prefix, drops every '..'/empty segment."""
    # webpack://, ng://, and similar devtools-only pseudo-schemes - strip
    # the scheme entirely, keeping only the path-like remainder.
    stripped = re.sub(r"^[a-zA-Z][a-zA-Z0-9+.-]*://", "", raw_path)
    stripped = stripped.lstrip("/")
    parts = [p for p in stripped.split("/") if p not in ("", ".", "..")]
    if not parts:
        return None
    return "/".join(parts)


def safe_join(base_dir, relative_path):
    base_real = os.path.realpath(base_dir)
    candidate = os.path.realpath(os.path.join(base_dir, relative_path))
    if candidate != base_real and not candidate.startswith(base_real + os.sep):
        return None
    return candidate


def scan_text_for_sensitive_patterns(text, origin):
    findings = []
    for category, pattern in SENSITIVE_PATTERNS:
        for match in pattern.finditer(text):
            findings.append(
                {
                    "category": category,
                    "match": match.group(0),
                    "source": origin,
                }
            )
    return findings


def scan_path_for_local_paths(path, origin):
    findings = []
    for category, pattern in LOCAL_PATH_PATTERNS:
        for match in pattern.finditer(path):
            findings.append(
                {
                    "category": category,
                    "match": match.group(0),
                    "source": origin,
                }
            )
    return findings


def process_sourcemap(sourcemap, map_origin, output_root, findings, index):
    sources = sourcemap.get("sources") or []
    sources_content = sourcemap.get("sourcesContent") or []
    source_root = sourcemap.get("sourceRoot") or ""

    for i, source in enumerate(sources):
        full_source = f"{source_root}{source}" if source_root else source
        findings.extend(scan_path_for_local_paths(full_source, map_origin))

        content = sources_content[i] if i < len(sources_content) else None
        record = {
            "map": map_origin,
            "original_source_path": full_source,
            "content_available": content is not None,
        }
        findings.append({"category": "source_discovered", "match": full_source, "source": map_origin})

        if content:
            findings.extend(scan_text_for_sensitive_patterns(content, f"{map_origin} :: {full_source}"))

            safe_relative = sanitize_source_path(full_source)
            if safe_relative:
                dest = safe_join(output_root, os.path.join(str(index), safe_relative))
                if dest:
                    os.makedirs(os.path.dirname(dest), exist_ok=True)
                    with open(dest, "w", encoding="utf-8", errors="replace") as f:
                        f.write(content)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--max-scripts", type=int, default=20, dest="max_scripts")
    parser.add_argument("--timeout", type=int, default=20)
    args = parser.parse_args()

    findings = []
    output_root = "/input/sources"
    os.makedirs(output_root, exist_ok=True)

    map_refs = []  # list of (map_ref_or_none, base_url, direct_json_or_none)

    if args.target.endswith(".map"):
        try:
            raw = fetch(args.target, args.timeout)
            map_refs.append((None, args.target, json.loads(raw)))
        except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, ValueError) as e:
            sys.stderr.write(f"error: could not fetch/parse source map {args.target}: {e}\n")
            sys.exit(1)
    elif args.target.endswith(".js"):
        try:
            js_text = fetch(args.target, args.timeout).decode("utf-8", errors="replace")
        except (urllib.error.URLError, urllib.error.HTTPError) as e:
            sys.stderr.write(f"error: could not fetch {args.target}: {e}\n")
            sys.exit(1)
        map_ref = find_sourcemap_url(js_text)
        if not map_ref:
            sys.stderr.write(f"no sourceMappingURL comment found in {args.target}\n")
            sys.exit(0)
        map_refs.append((map_ref, args.target, None))
    else:
        try:
            html_text = fetch(args.target, args.timeout).decode("utf-8", errors="replace")
        except (urllib.error.URLError, urllib.error.HTTPError) as e:
            sys.stderr.write(f"error: could not fetch {args.target}: {e}\n")
            sys.exit(1)
        script_parser = ScriptTagParser()
        script_parser.feed(html_text)
        script_urls = [urllib.parse.urljoin(args.target, src) for src in script_parser.srcs][: args.max_scripts]
        if not script_urls:
            sys.stderr.write(f"no <script src> tags found on {args.target}\n")
            sys.exit(0)
        for script_url in script_urls:
            try:
                js_text = fetch(script_url, args.timeout).decode("utf-8", errors="replace")
            except (urllib.error.URLError, urllib.error.HTTPError):
                continue
            map_ref = find_sourcemap_url(js_text)
            if map_ref:
                map_refs.append((map_ref, script_url, None))

    if not map_refs:
        sys.stderr.write("no source maps discovered\n")
        sys.exit(0)

    processed_any = False
    for index, (map_ref, base_url, direct_json) in enumerate(map_refs):
        if direct_json is not None:
            sourcemap, map_origin = direct_json, base_url
        else:
            sourcemap, map_origin = load_sourcemap(map_ref, base_url, args.timeout)
        if sourcemap is None:
            findings.append(
                {"category": "sourcemap_fetch_failed", "match": map_ref or base_url, "source": base_url}
            )
            continue
        processed_any = True
        process_sourcemap(sourcemap, map_origin, output_root, findings, index)

    with open("/artifacts/findings.json", "w") as f:
        json.dump(findings, f, indent=2)

    print(json.dumps(findings, indent=2))
    sys.exit(0 if processed_any or findings else 1)


if __name__ == "__main__":
    main()
