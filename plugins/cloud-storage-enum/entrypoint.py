#!/usr/bin/env python3
"""Thin wrapper around cloud_enum's own real storage-bucket-check
functions (github.com/initstring/cloud_enum, MIT license), pinned to the
exact commit d3dba3716e10262f0c18a30f381a5eb617d7afd5 - both PyPI's own
published package (0.0.1) and the latest git tag (v0.2) are stale
relative to the real current source's own pyproject.toml version (0.8),
confirmed live rather than assumed, so pinned by commit instead of
either.

Imports and calls the library's own check_s3_buckets/check_gcp_buckets/
check_storage_accounts/brute_force_containers functions directly, rather
than invoking cloud_enum's full CLI - its own run_all() per provider also
probes several unrelated non-storage services (Firebase RTDB, App
Engine, Cloud Functions, AWS Apps, Azure VMs/websites/databases/Key
Vault) with no flag to scope those out, which is out of this plugin's
stated storage-only scope.
"""
import argparse
import json
import os
import sys

import cloud_enum as ce
from enum_tools import utils, aws_checks, gcp_checks, azure_checks

FUZZ_FILE = "/opt/fuzz.txt"
FINDINGS_PATH = "/artifacts/findings.jsonl"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--keyword", required=True)
    parser.add_argument("--threads", type=int, default=5)
    parser.add_argument("--nameserver", default="1.1.1.1")
    parser.add_argument("--disable-aws", action="store_true")
    parser.add_argument("--disable-azure", action="store_true")
    parser.add_argument("--disable-gcp", action="store_true")
    parser.add_argument("--quickscan", action="store_true")
    args = parser.parse_args()

    keywords = [k.strip() for k in args.keyword.split(",") if k.strip()]
    if not keywords:
        sys.stderr.write("error: no valid keyword given\n")
        sys.exit(2)

    utils.init_logfile(FINDINGS_PATH, "json")

    mutations = [] if args.quickscan else ce.read_mutations(FUZZ_FILE)
    names = ce.build_names(keywords, mutations)

    if not args.disable_aws:
        aws_checks.check_s3_buckets(names, args.threads)
    if not args.disable_gcp:
        gcp_checks.check_gcp_buckets(names, args.threads)
    if not args.disable_azure:
        storage_accounts = azure_checks.check_storage_accounts(
            names, args.threads, args.nameserver, False
        )
        if storage_accounts:
            azure_checks.brute_force_containers(storage_accounts, FUZZ_FILE, args.threads)

    findings = []
    if os.path.exists(FINDINGS_PATH):
        with open(FINDINGS_PATH) as f:
            for line in f:
                line = line.strip()
                if line.startswith("{"):
                    try:
                        findings.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass

    print(json.dumps(findings, indent=2))


if __name__ == "__main__":
    main()
