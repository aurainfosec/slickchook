import argparse, json, ssl, urllib.request

parser = argparse.ArgumentParser()
parser.add_argument("--url", required=True)
args = parser.parse_args()
query = {"query": "query Introspection { __schema { queryType { name } types { name } } }"}
request = urllib.request.Request(args.url, data=json.dumps(query).encode(), headers={"Content-Type": "application/json"})
try:
    with urllib.request.urlopen(request, context=ssl._create_unverified_context(), timeout=20) as response:
        raw = response.read()
        try:
            body = json.loads(raw)
        except json.JSONDecodeError:
            body = raw.decode("utf-8", "replace")[:4096]
        print(json.dumps({"status": response.status, "content_type": response.headers.get("content-type"), "body": body}))
except Exception as error:
    print(json.dumps({"error": str(error)}))
    raise SystemExit(1)
