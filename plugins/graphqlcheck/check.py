import argparse, json, re, ssl, urllib.request, urllib.parse

parser = argparse.ArgumentParser()
parser.add_argument("--url", required=True)
parser.add_argument("--guess-fields", action="store_true")
parser.add_argument("--wordlist", default="/fields.txt")
args = parser.parse_args()


def post_query(url, query):
    request = urllib.request.Request(
        url,
        data=json.dumps({"query": query}).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, context=ssl._create_unverified_context(), timeout=20) as response:
        raw = response.read()
        try:
            return response.status, json.loads(raw)
        except json.JSONDecodeError:
            return response.status, raw.decode("utf-8", "replace")[:2048]


def guess_fields(url, wordlist_path):
    # Differential error-message technique: a field that doesn't exist gets a
    # "Cannot query field"/"Unknown field" error; a field that exists but was
    # queried wrong (missing arguments, wrong sub-selection) errors some other
    # way, or succeeds outright. That distinction is the only signal here -
    # this is heuristic, not confirmed schema, and is only useful when real
    # introspection is disabled.
    unknown_field_re = re.compile(r"cannot query field|unknown field", re.I)
    try:
        with open(wordlist_path, encoding="utf-8") as handle:
            candidates = [line.strip() for line in handle if line.strip()]
    except OSError as error:
        return {"error": f"could not read wordlist {wordlist_path}: {error}"}

    findings = {}
    for name in candidates:
        try:
            _, body = post_query(url, "{ " + name + " { __typename } }")
        except Exception as error:
            findings[name] = f"request_failed: {error}"
            continue
        errors = body.get("errors") if isinstance(body, dict) else None
        if not errors:
            findings[name] = "exists (query succeeded)"
            continue
        messages = " ".join(str(e.get("message", "")) for e in errors if isinstance(e, dict))
        if unknown_field_re.search(messages):
            continue  # does not exist on the root Query type; omit to keep output focused
        findings[name] = f"likely exists (error was not \"unknown field\": {messages[:200]})"
    return findings


query = "query Introspection { __schema { queryType { name } types { name } } }"
path = urllib.parse.urlparse(args.url).path.lower()
# Static assets must be fetched with GET; POSTing an introspection body causes
# many reverse proxies/SPA routers to return index.html instead of the asset.
if path.endswith((".js", ".mjs", ".json", ".map", ".css", ".webmanifest")):
    request = urllib.request.Request(args.url, method="GET")
else:
    request = urllib.request.Request(args.url, data=json.dumps({"query": query}).encode(), headers={"Content-Type": "application/json"})
try:
    with urllib.request.urlopen(request, context=ssl._create_unverified_context(), timeout=20) as response:
        raw = response.read()
        try:
            body = json.loads(raw)
        except json.JSONDecodeError:
            body = raw.decode("utf-8", "replace")[:4096]
        content_type = (response.headers.get("content-type") or "").split(";", 1)[0].lower()
        anomalies = []
        if path.endswith((".js", ".mjs")) and content_type not in ("application/javascript", "text/javascript"):
            anomalies.append("script extension returned non-JavaScript content type")
        if path.endswith((".json", ".map")) and content_type not in ("application/json", "application/manifest+json"):
            anomalies.append("JSON extension returned non-JSON content type")
        if content_type == "text/html" and isinstance(body, str) and "<app-root" in body.lower():
            anomalies.append("response is the Angular SPA fallback shell")
        result = {"status": response.status, "content_type": content_type, "body": body}
        if anomalies:
            result["static_analysis_warnings"] = anomalies
        if args.guess_fields:
            result["guessed_fields"] = guess_fields(args.url, args.wordlist)
            result["guessed_fields_note"] = (
                "Heuristic, error-message-based field guessing against a small bundled "
                "wordlist - only useful when introspection above is disabled/restricted. "
                "Treat as an untrusted signal to confirm manually, not a confirmed schema."
            )
        print(json.dumps(result))
except Exception as error:
    # The initial request itself failed (connection refused, DNS failure,
    # TLS error, timeout) - the URL is unreachable, not just introspection-
    # disabled, so a 40-candidate guessing pass against the same URL would
    # only burn the run's whole timeout budget hitting the same failure
    # again and again. Skip it here; guess_fields only runs on the success
    # path above, where the target is confirmed reachable.
    print(json.dumps({"error": str(error)}))
    raise SystemExit(1)
