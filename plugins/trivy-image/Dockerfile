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

# Trivy's own scan cache (recording per-run analysis results, distinct
# from the read-mostly vulnerability DB) is stored in the same
# --cache-dir - confirmed from its own docs, not assumed. This fleet's
# containers always run --read-only, so the baked-in template above
# can't be written to directly at runtime. entrypoint.sh copies it into
# writable tmpfs on every run instead - the same pattern already used by
# the jwt-analyzer plugin for a different read-only-filesystem
# constraint (a tool needing a second, writable invocation of state
# baked in at build time).
COPY entrypoint.sh /usr/local/bin/slickchook-trivy-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-trivy-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/slickchook-trivy-entrypoint.sh"]
