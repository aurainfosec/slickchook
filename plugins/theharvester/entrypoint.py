#!/usr/bin/env python3
"""Writes theHarvester's own api-keys.yaml from injected [[secrets]] env
vars, then execs the real tool.

theHarvester reads API keys *only* from a local YAML config file
(searched at ~/.theHarvester/api-keys.yaml first) - confirmed directly
from its own lib/core.py, which has zero environment-variable fallback
anywhere in the load path. This mirrors a gap found (but left unwired)
in the waymore plugin - built here instead, since credential reuse
across plugins was explicitly requested.

Where an already-installed plugin already uses the same underlying
provider, this reuses that exact secret name rather than declaring a
separate, redundant one: SHODAN_API_KEY (shodan-host/shodan-search),
VT_API_KEY (virustotal-resolutions), GH_TOKEN (github-code-search),
BRAVE_API_KEY (brave-dork-search), DEHASHED_API_KEY (dehashed). Every
other source theHarvester supports a key for gets its own
THEHARVESTER_*-prefixed secret, since no other installed plugin covers
that provider.

Every field is optional - theHarvester works fine with only its free/
keyless sources if nothing is configured, and --quiet (always passed by
the manifest) suppresses its own missing-key warnings for the rest.
"""
import os
import sys

HOME = "/tmp"
CONFIG_DIR = os.path.join(HOME, ".theHarvester")
CONFIG_PATH = os.path.join(CONFIG_DIR, "api-keys.yaml")

# (yaml path segments) -> env var name
FIELDS = [
    (("shodan", "key"), "SHODAN_API_KEY"),
    (("virustotal", "key"), "VT_API_KEY"),
    (("github", "key"), "GH_TOKEN"),
    (("brave", "key"), "BRAVE_API_KEY"),
    (("dehashed", "key"), "DEHASHED_API_KEY"),
    (("bevigil", "key"), "THEHARVESTER_BEVIGIL_KEY"),
    (("bitbucket", "key"), "THEHARVESTER_BITBUCKET_KEY"),
    (("bufferoverun", "key"), "THEHARVESTER_BUFFEROVERUN_KEY"),
    (("builtwith", "key"), "THEHARVESTER_BUILTWITH_KEY"),
    (("censys", "id"), "THEHARVESTER_CENSYS_ID"),
    (("censys", "secret"), "THEHARVESTER_CENSYS_SECRET"),
    (("criminalip", "key"), "THEHARVESTER_CRIMINALIP_KEY"),
    (("dnsdumpster", "key"), "THEHARVESTER_DNSDUMPSTER_KEY"),
    (("dymo", "key"), "THEHARVESTER_DYMO_KEY"),
    (("fofa", "key"), "THEHARVESTER_FOFA_KEY"),
    (("fofa", "email"), "THEHARVESTER_FOFA_EMAIL"),
    (("fullhunt", "key"), "THEHARVESTER_FULLHUNT_KEY"),
    (("hackertarget", "key"), "THEHARVESTER_HACKERTARGET_KEY"),
    (("haveibeenpwned", "key"), "THEHARVESTER_HAVEIBEENPWNED_KEY"),
    (("hunter", "key"), "THEHARVESTER_HUNTER_KEY"),
    (("hunterhow", "key"), "THEHARVESTER_HUNTERHOW_KEY"),
    (("intelx", "key"), "THEHARVESTER_INTELX_KEY"),
    (("leakix", "key"), "THEHARVESTER_LEAKIX_KEY"),
    (("leaklookup", "key"), "THEHARVESTER_LEAKLOOKUP_KEY"),
    (("mojeek", "key"), "THEHARVESTER_MOJEEK_KEY"),
    (("netlas", "key"), "THEHARVESTER_NETLAS_KEY"),
    (("onyphe", "key"), "THEHARVESTER_ONYPHE_KEY"),
    (("pentestTools", "key"), "THEHARVESTER_PENTESTTOOLS_KEY"),
    (("projectDiscovery", "key"), "THEHARVESTER_PROJECTDISCOVERY_KEY"),
    (("rocketreach", "key"), "THEHARVESTER_ROCKETREACH_KEY"),
    (("securityscorecard", "key"), "THEHARVESTER_SECURITYSCORECARD_KEY"),
    (("securityTrails", "key"), "THEHARVESTER_SECURITYTRAILS_KEY"),
    (("sherlockeye", "key"), "THEHARVESTER_SHERLOCKEYE_KEY"),
    (("tomba", "key"), "THEHARVESTER_TOMBA_KEY"),
    (("tomba", "secret"), "THEHARVESTER_TOMBA_SECRET"),
    (("venacus", "key"), "THEHARVESTER_VENACUS_KEY"),
    (("whoisxml", "key"), "THEHARVESTER_WHOISXML_KEY"),
    (("windvane", "key"), "THEHARVESTER_WINDVANE_KEY"),
    (("zoomeye", "key"), "THEHARVESTER_ZOOMEYE_KEY"),
]


def yaml_quote(value):
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def main():
    os.makedirs(CONFIG_DIR, exist_ok=True)

    sources = {}
    for (source, field), env_var in FIELDS:
        value = os.environ.get(env_var)
        if value:
            sources.setdefault(source, {})[field] = value

    lines = ["apikeys:", ""]
    for source, fields in sources.items():
        lines.append(f"  {source}:")
        for field, value in fields.items():
            lines.append(f"    {field}: {yaml_quote(value)}")
        lines.append("")

    with open(CONFIG_PATH, "w") as f:
        f.write("\n".join(lines) + "\n")

    os.environ["HOME"] = HOME
    os.execvp("theHarvester", ["theHarvester"] + sys.argv[1:])


if __name__ == "__main__":
    main()
