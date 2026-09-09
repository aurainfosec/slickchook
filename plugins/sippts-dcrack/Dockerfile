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
