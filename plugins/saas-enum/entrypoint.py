#!/usr/bin/env python3
"""Checks whether an organization has a live presence on common enterprise
SaaS platforms via their predictable branded vanity-subdomain patterns
(e.g. {org}.slack.com, {org}.atlassian.net) - reveals what
helpdesk/HR/CRM/collaboration systems an org actually runs.

Only platforms independently confirmed (by live-testing a real customer's
vanity URL against an obviously-nonexistent one) to give an actually
distinguishable "exists" vs "doesn't exist" signal are checked here. Many
platforms - Okta and Freshdesk among them - serve the exact same response
regardless of whether an org exists, and several commonly-assumed patterns
(Greenhouse, Lever, BambooHR, DocuSign, Workday, Asana) turned out to be
wrong or unverifiable when actually tested live rather than assumed from
memory. Shipping a check that always says "maybe" is worse than not
shipping it - excluded platforms are simply not checked, not silently
included as noise. See this plugin's own description for the excluded
list and why.
"""
import argparse
import json
import sys
import urllib.error
import urllib.request

TIMEOUT_DEFAULT = 15


def fetch(url, timeout):
    """Returns (status, final_url) for any completed HTTP response
    (following redirects), whether 2xx or an HTTPError status. Lets a
    genuine connection-level failure (DNS resolution failure, connection
    refused, timeout) propagate as urllib.error.URLError (not HTTPError)."""
    request = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return response.status, response.geturl()
    except urllib.error.HTTPError as e:
        return e.code, e.geturl() or url


def check_zendesk(org, timeout):
    url = f"https://{org}.zendesk.com/"
    try:
        status, _ = fetch(url, timeout)
    except urllib.error.URLError:
        return "not_found", url, None
    if status == 200:
        return "exists", url, None
    if status == 403:
        return "not_found", url, None
    return "inconclusive", url, f"unexpected status {status}"


def check_atlassian(org, timeout):
    url = f"https://{org}.atlassian.net/"
    try:
        status, final_url = fetch(url, timeout)
    except urllib.error.URLError:
        return "not_found", url, None
    if status in (200, 202):
        return "exists", final_url, None
    if status == 404:
        return "not_found", url, None
    return "inconclusive", url, f"unexpected status {status}"


def check_slack(org, timeout):
    url = f"https://{org}.slack.com/"
    try:
        status, _ = fetch(url, timeout)
    except urllib.error.URLError:
        return "not_found", url, None
    if status == 403:
        return "exists", url, None
    if status == 404:
        return "not_found", url, None
    return "inconclusive", url, f"unexpected status {status}"


def check_pagerduty(org, timeout):
    url = f"https://{org}.pagerduty.com/"
    try:
        status, final_url = fetch(url, timeout)
    except urllib.error.URLError:
        return "not_found", url, None
    if status == 404:
        return "not_found", url, None
    if status == 200:
        return "exists", final_url, None
    return "inconclusive", url, f"unexpected status {status}"


def check_monday(org, timeout):
    # The reliable signal here is purely whether the final URL names
    # "slug_not_found" - a real org's response can land on more than one
    # legitimate status/redirect target (a login page, a 406 challenge,
    # etc), so status code alone isn't a safe basis for this one (caught
    # by testing against a real org before shipping, not assumed).
    url = f"https://{org}.monday.com/"
    try:
        status, final_url = fetch(url, timeout)
    except urllib.error.URLError:
        return "not_found", url, None
    if "slug_not_found" in final_url:
        return "not_found", final_url, None
    return "exists", final_url, None


def check_connectivity_only(org, domain_template, timeout):
    """For platforms where a real org's vanity URL resolves and responds
    (with any status) but a nonexistent one fails to connect/resolve at
    all - the signal is purely "did a connection complete", not any
    particular status code."""
    url = domain_template.format(org=org)
    try:
        fetch(url, timeout)
        return "exists", url, None
    except urllib.error.URLError:
        return "not_found", url, None


def check_shopify(org, timeout):
    url = f"https://{org}.myshopify.com/"
    try:
        status, final_url = fetch(url, timeout)
    except urllib.error.URLError:
        return "not_found", url, None
    if status == 200:
        return "exists", final_url, None
    if status == 404:
        return "not_found", url, None
    return "inconclusive", url, f"unexpected status {status}"


def check_gitbook(org, timeout):
    url = f"https://{org}.gitbook.io/"
    try:
        status, final_url = fetch(url, timeout)
    except urllib.error.URLError:
        return "not_found", url, None
    if status == 404:
        return "not_found", url, None
    if status in (200, 307) and f"{org}.gitbook.io" not in final_url:
        return (
            "exists_unverified_attribution",
            final_url,
            "resolved and redirected to a custom domain - GitBook allows any "
            "org name to be claimed, so this does not confirm the redirect "
            "target actually belongs to the organization being investigated",
        )
    return "inconclusive", url, f"status {status}, final_url={final_url}"


PLATFORMS = {
    "zendesk": check_zendesk,
    "atlassian": check_atlassian,
    "slack": check_slack,
    "pagerduty": check_pagerduty,
    "monday": check_monday,
    "zoom": lambda org, timeout: check_connectivity_only(org, "https://{org}.zoom.us/", timeout),
    "sharepoint": lambda org, timeout: check_connectivity_only(org, "https://{org}.sharepoint.com/", timeout),
    "salesforce": lambda org, timeout: check_connectivity_only(org, "https://{org}.my.salesforce.com/", timeout),
    "gitbook": check_gitbook,
    "shopify": check_shopify,
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--org", required=True)
    parser.add_argument("--platforms", default="")
    parser.add_argument("--timeout", type=int, default=TIMEOUT_DEFAULT)
    args = parser.parse_args()

    if args.platforms:
        requested = [p.strip() for p in args.platforms.split(",") if p.strip()]
        unknown = [p for p in requested if p not in PLATFORMS]
        if unknown:
            sys.stderr.write(
                f"error: unknown platform(s) {', '.join(unknown)}. "
                f"Available: {', '.join(sorted(PLATFORMS))}\n"
            )
            sys.exit(2)
        selected = requested
    else:
        selected = sorted(PLATFORMS)

    results = []
    for platform in selected:
        result, url, note = PLATFORMS[platform](args.org, args.timeout)
        entry = {"platform": platform, "url": url, "result": result}
        if note:
            entry["note"] = note
        results.append(entry)

    with open("/artifacts/findings.json", "w") as f:
        json.dump(results, f, indent=2)

    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
