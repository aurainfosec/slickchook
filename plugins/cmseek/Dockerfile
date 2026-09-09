# CMSeeK has no meaningful version tags - its latest git tag (v.1.1.3) is
# frozen at a 2020 commit even though master keeps receiving real,
# ongoing community fixes and new CMS signatures (confirmed live via its
# own commit history, most recently July 2026) - pinned to that exact
# current commit instead of the stale tag.
FROM python:3.13-slim

RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN git clone https://github.com/Tuhinshubhra/CMSeeK.git /opt/cmseek \
    && cd /opt/cmseek && git checkout 03a6e7f8
WORKDIR /opt/cmseek
RUN pip install --no-cache-dir -r requirements.txt

# CMSeeK writes its results under its own current working directory, not
# a configurable output path (confirmed from its own source,
# cmseekdb/basic.py: it uses its installed directory only if that's
# writable, and falls back to os.getcwd() otherwise - this fleet's
# containers run --read-only, so /opt/cmseek won't be writable at
# runtime). Setting the runtime working directory to /artifacts (a
# different, writable path) makes it take that fallback silently and
# correctly, with no prompt of any kind.
WORKDIR /artifacts

ENTRYPOINT ["python3", "/opt/cmseek/cmseek.py"]
