# The real official image (confirmed from its own Dockerfile at this
# exact tag) uses CMD, not ENTRYPOINT, and chown's the binary to a
# named non-root user - neither works with how this fleet invokes
# containers (declared options/arguments become the command, replacing
# a bare CMD entirely; and this fleet always overrides the container
# user to UID 65534, which wouldn't necessarily have execute permission
# on a file owned by a different, unrelated user). Fixed with an
# explicit ENTRYPOINT and a world-readable/executable permission fix.
FROM hahwul/dalfox:v3.2.1
USER root
RUN chmod a+rX /app/dalfox
ENTRYPOINT ["/app/dalfox"]
