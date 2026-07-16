import argparse, json, ssl, urllib.request, urllib.parse

parser = argparse.ArgumentParser()
parser.add_argument("--url", required=True)
args = parser.parse_args()
query = {"query": "query Introspection { __schema { queryType { name } types { name } } }"}
path = urllib.parse.urlparse(args.url).path.lower()
# Static assets must be fetched with GET; POSTing an introspection body causes
# many reverse proxies/SPA routers to return index.html instead of the asset.
if path.endswith((".js", ".mjs", ".json", ".map", ".css", ".webmanifest")):
    request = urllib.request.Request(args.url, method="GET")
else:
    request = urllib.request.Request(args.url, data=json.dumps(query).encode(), headers={"Content-Type": "application/json"})
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
        print(json.dumps(result))
except Exception as error:
    print(json.dumps({"error": str(error)}))
    raise SystemExit(1)
