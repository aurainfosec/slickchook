# Bake the vulnerability DB and Java index DB into the image at build
# time (real, documented flags: --download-db-only/--download-java-db-only),
# rather than fetch them on every ephemeral run - matching this fleet's
# existing osvscanner precedent ("refresh during image builds, runtime
# scanning remains offline"). Misconfiguration/license scanning (which
# need a separately-fetched checks bundle with no equivalent pre-fetch-
# only flag, confirmed from Trivy's own docs) are deliberately out of
# scope here - scanners stays limited to Trivy's own real default for
# image scanning (vuln, secret), neither of which needs that bundle.
FROM aquasec/trivy:0.74.0

RUN trivy image --cache-dir /opt/trivy-db-template --download-db-only \
    && trivy image --cache-dir /opt/trivy-db-template --download-java-db-only

# Confirmed live: the baked-in template above is a real 2.7GB (1.3GB DB +
# 1.4GB Java DB) - too large to copy into tmpfs on every run without a
# heavy memory/latency cost. Trivy's own scan cache (the only part that
# actually needs to be writable, distinct from the read-mostly DB
# itself) has a documented in-memory backend (--cache-backend memory,
# set in this plugin's fixed_args) - using it means --cache-dir can
# point directly at this read-only baked-in template with no copy step
# and no wrapper entrypoint needed at all.
ENTRYPOINT ["trivy"]
