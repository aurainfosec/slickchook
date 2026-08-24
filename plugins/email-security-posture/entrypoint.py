#!/usr/bin/env python3
"""Checks an authorized domain's SPF, DKIM, and DMARC DNS records for
common misconfigurations - purely passive DNS lookups, no contact with
the domain's actual mail infrastructure at all.

DKIM records have no canonical, discoverable location - the selector is
chosen by whoever configured the domain's mail provider, and there is no
DNS mechanism to enumerate them. This checks a list of selectors publicly
documented as defaults by widely-used providers (Google Workspace,
Microsoft 365, SendGrid, Mailchimp/Mandrill, Constant Contact,
ProtonMail) plus several generic self-hosted conventions - this is
inherently a best-effort guess, not an exhaustive/authoritative check.
The only fully reliable way to know a domain's real DKIM selector is
from an actual email's DKIM-Signature header - pass one via --selectors
if known. Confirmed live that Microsoft 365's real DKIM records are
often published as a CNAME to a `<selector>-<domain>._domainkey.
<tenant>.onmicrosoft.com` target rather than a direct TXT record - a
standard TXT-type resolution already follows that transparently, no
special handling needed.
"""
import argparse
import json
import re
import sys

import dns.resolver
import dns.exception

COMMON_DKIM_SELECTORS = [
    "google", "selector1", "selector2", "k1", "k2", "s1", "s2",
    "dkim", "default", "mail", "smtp", "mx",
    "everlytickey1", "everlytickey2", "zoho", "mandrill",
    "protonmail", "protonmail2", "protonmail3",
]

LOOKUP_MECHANISM_RE = re.compile(r"(?i)\b(include|a|mx|ptr|exists|redirect)\b")


def get_txt_records(name, resolver):
    try:
        answers = resolver.resolve(name, "TXT")
        return [b"".join(r.strings).decode("utf-8", errors="replace") for r in answers]
    except (dns.resolver.NXDOMAIN, dns.resolver.NoAnswer, dns.exception.Timeout, dns.resolver.NoNameservers):
        return []


def get_mx_records(name, resolver):
    try:
        answers = resolver.resolve(name, "MX")
        return sorted(f"{r.preference} {str(r.exchange).rstrip('.')}" for r in answers)
    except (dns.resolver.NXDOMAIN, dns.resolver.NoAnswer, dns.exception.Timeout, dns.resolver.NoNameservers):
        return []


def analyze_spf(txt_records):
    findings = []
    spf_records = [r for r in txt_records if re.match(r"(?i)^v=spf1\b", r)]
    if not spf_records:
        findings.append({"check": "spf_present", "status": "fail", "detail": "No SPF record found."})
        return findings
    if len(spf_records) > 1:
        findings.append({
            "check": "spf_single_record", "status": "fail",
            "detail": f"{len(spf_records)} SPF records found - RFC 7208 requires exactly one; "
                      "multiple records make SPF evaluation fail (permerror) for all senders.",
        })
    record = spf_records[0]
    findings.append({"check": "spf_present", "status": "pass", "detail": record})

    if re.search(r"(?i)(^|\s)-all\b", record):
        findings.append({"check": "spf_all_mechanism", "status": "pass",
                          "detail": "Ends in -all (hardfail) - non-matching senders should be rejected by a compliant receiver."})
    elif re.search(r"(?i)(^|\s)~all\b", record):
        findings.append({"check": "spf_all_mechanism", "status": "info",
                          "detail": "Ends in ~all (softfail) - non-matching senders are marked/flagged, not rejected outright."})
    elif re.search(r"(?i)(^|\s)\?all\b", record):
        findings.append({"check": "spf_all_mechanism", "status": "warn",
                          "detail": "Ends in ?all (neutral) - provides no real enforcement either way."})
    elif re.search(r"(?i)(^|\s)\+?all\b", record):
        findings.append({"check": "spf_all_mechanism", "status": "fail",
                          "detail": "Ends in (+)all (pass-all, the '+' is implicit if omitted) - explicitly allows ANY server "
                                    "to send mail as this domain; provides no protection at all."})
    else:
        findings.append({"check": "spf_all_mechanism", "status": "warn",
                          "detail": "No 'all' mechanism found - behavior for non-matching senders is undefined."})

    lookup_count = len(LOOKUP_MECHANISM_RE.findall(record))
    if lookup_count > 10:
        findings.append({
            "check": "spf_lookup_limit", "status": "fail",
            "detail": f"~{lookup_count} DNS-lookup-causing mechanisms found (include/a/mx/ptr/exists/redirect) - "
                      "RFC 7208 caps SPF evaluation at 10 lookups; exceeding it causes a permerror, which many "
                      "receivers treat as an outright SPF failure. This is an approximate count (each 'include' "
                      "chain's own nested lookups aren't recursively resolved here) - the real total may be higher.",
        })
    return findings


def analyze_dmarc(txt_records):
    findings = []
    dmarc_records = [r for r in txt_records if re.match(r"(?i)^v=DMARC1\b", r)]
    if not dmarc_records:
        findings.append({"check": "dmarc_present", "status": "fail", "detail": "No DMARC record found at _dmarc.<domain>."})
        return findings
    record = dmarc_records[0]
    findings.append({"check": "dmarc_present", "status": "pass", "detail": record})

    policy_match = re.search(r"(?i)\bp=(none|quarantine|reject)\b", record)
    policy = policy_match.group(1).lower() if policy_match else None
    if policy == "none":
        findings.append({"check": "dmarc_policy", "status": "warn",
                          "detail": "Policy is p=none - failing messages are reported on but not acted on at all; monitoring-only."})
    elif policy == "quarantine":
        findings.append({"check": "dmarc_policy", "status": "info",
                          "detail": "Policy is p=quarantine - failing messages should be spam-foldered, not rejected outright."})
    elif policy == "reject":
        findings.append({"check": "dmarc_policy", "status": "pass",
                          "detail": "Policy is p=reject - failing messages should be rejected outright by a compliant receiver."})
    else:
        findings.append({"check": "dmarc_policy", "status": "warn", "detail": "No recognizable p= policy tag found."})

    pct_match = re.search(r"(?i)\bpct=(\d+)\b", record)
    if pct_match and int(pct_match.group(1)) < 100:
        findings.append({
            "check": "dmarc_pct", "status": "warn",
            "detail": f"pct={pct_match.group(1)} - only that percentage of failing mail is subject to the stated "
                      "policy; the rest is treated as if policy were p=none.",
        })

    if not re.search(r"(?i)\brua=", record):
        findings.append({"check": "dmarc_rua", "status": "info",
                          "detail": "No rua= aggregate-report address configured - no visibility into who is "
                                    "(or isn't) passing DMARC for this domain."})

    return findings


def check_dkim(domain, selectors, resolver):
    results = []
    for selector in selectors:
        name = f"{selector}._domainkey.{domain}"
        txt_records = get_txt_records(name, resolver)
        dkim_records = [r for r in txt_records if re.match(r"(?i)^v=DKIM1\b", r) or re.search(r"(?i)\bp=", r)]
        if dkim_records:
            results.append({"selector": selector, "record": dkim_records[0]})
    return results


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--domain", required=True)
    parser.add_argument("--selectors", default="")
    parser.add_argument("--nameserver", default="1.1.1.1")
    parser.add_argument("--timeout", type=int, default=10)
    args = parser.parse_args()

    resolver = dns.resolver.Resolver()
    resolver.nameservers = [args.nameserver]
    resolver.timeout = args.timeout
    resolver.lifetime = args.timeout

    custom_selectors = [s.strip() for s in args.selectors.split(",") if s.strip()]
    selectors = custom_selectors + [s for s in COMMON_DKIM_SELECTORS if s not in custom_selectors]

    spf_txt = get_txt_records(args.domain, resolver)
    dmarc_txt = get_txt_records(f"_dmarc.{args.domain}", resolver)

    result = {
        "domain": args.domain,
        "mx_records": get_mx_records(args.domain, resolver),
        "spf": analyze_spf(spf_txt),
        "dmarc": analyze_dmarc(dmarc_txt),
        "dkim_selectors_checked": selectors,
        "dkim_found": check_dkim(args.domain, selectors, resolver),
    }

    with open("/artifacts/findings.json", "w") as f:
        json.dump(result, f, indent=2)

    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
