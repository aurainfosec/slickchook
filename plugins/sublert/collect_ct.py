import json, sys, urllib.parse, urllib.request

domain = sys.argv[1] if len(sys.argv) > 1 else ""
if not domain or any(c in domain for c in "/\\ \t\r\n"):
    raise SystemExit("invalid domain")
url = "https://crt.sh/?q=" + urllib.parse.quote("%." + domain) + "&output=json"
with urllib.request.urlopen(url, timeout=30) as response:
    rows = json.load(response)
names = set()
for row in rows:
    for name in row.get("name_value", "").splitlines():
        name = name.strip().lower()
        if name and name.endswith(domain.lower()):
            names.add(name)
for name in sorted(names):
    print(name)
