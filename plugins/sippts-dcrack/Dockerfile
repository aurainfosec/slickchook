# Same real upstream and same two real fixes as the sippts-scan plugin:
# pyproject.toml's dynamic dependencies point at a requirements.txt that
# doesn't actually exist in this tag (confirmed live), and netifaces (a
# transitive dependency) has no prebuilt wheel here and needs a C
# compiler to build from source (also confirmed live, not assumed).
FROM python:3.13-slim

RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates build-essential \
    && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch v4.1.2 https://github.com/Pepelux/sippts.git /opt/sippts
WORKDIR /opt/sippts
RUN printf 'netifaces\nrequests\nIPy\nscapy\npyshark\nwebsocket-client\nrel\n' > requirements.txt \
    && pip install --no-cache-dir .

# Real, confirmed upstream bug in sipdigestcrack.py's save_data(): the
# bruteforce-mode success-record format string is missing one `:%s`
# placeholder relative to its 6-element tuple (its wordlist-mode sibling,
# two lines above, correctly has all six) - `"bf:%s:%s:%s:%s:%s" % (...,
# 6 args)` raises `TypeError: not all arguments converted during string
# formatting`. That happens inside crack()'s own bare `except: pass`, so
# a successful bruteforce crack silently reports as "not found" instead -
# confirmed live by reproducing this exact failure locally, independent
# of Docker, before ever building this image. Patched at build time by
# adding the missing placeholder to that one exact line (its two
# already-correct 6-placeholder siblings are untouched, since the sed
# pattern only matches the shorter, broken string).
RUN sed -i 's/pl = "bf:%s:%s:%s:%s:%s" % (/pl = "bf:%s:%s:%s:%s:%s:%s" % (/' \
    "$(python3 -c 'import sippts.sipdigestcrack as m; print(m.__file__)')"

COPY entrypoint.py /usr/local/bin/slickchook-sippts-dcrack-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-sippts-dcrack-entrypoint.py

# dcrack writes a resume-cache file (sipdigestcrack.res) to its own
# current working directory - confirmed from source, sipdigestcrack.py's
# save_file()/save_data() - meaningless in this fleet's fresh-container-
# per-run model, but the working directory still needs to be writable
# for it to write there without crashing. /tmp is writable (this
# fleet's containers run --read-only) and keeps this leftover file out
# of /artifacts entirely.
WORKDIR /tmp

ENTRYPOINT ["/usr/local/bin/slickchook-sippts-dcrack-entrypoint.py"]
