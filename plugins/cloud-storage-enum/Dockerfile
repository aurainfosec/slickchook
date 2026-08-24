# PyPI's own published cloud_enum package (0.0.1) and the latest git tag
# (v0.2) are both stale relative to the real current source's own
# pyproject.toml version (0.8) - confirmed live, not assumed - so this
# installs directly from a pinned commit archive instead of either.
FROM python:3.13-slim
RUN pip install --no-cache-dir \
    "https://github.com/initstring/cloud_enum/archive/d3dba3716e10262f0c18a30f381a5eb617d7afd5.zip"
# fuzz.txt isn't declared as package data anywhere in the upstream repo
# (no MANIFEST.in, no package_data/include-package-data in
# pyproject.toml) - confirmed it wouldn't be bundled by pip install, so
# it's fetched separately at the exact same pinned commit and committed
# into this plugin's own directory (see fuzz.txt here), matching this
# fleet's usual bundled-wordlist-as-asset convention.
COPY --chmod=0644 fuzz.txt /opt/fuzz.txt
COPY entrypoint.py /usr/local/bin/slickchook-cloud-storage-enum-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-cloud-storage-enum-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-cloud-storage-enum-entrypoint.py"]
