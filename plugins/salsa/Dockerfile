# SALSA has no console-script/binary distribution - a real, pre-built
# release JAR is attached directly to its GitHub release (confirmed via
# the release's own assets, not built from source here). Pinned to
# salsa-1.0.1, its latest real release - confirmed live that master's
# few commits past this tag are only CI/release-plumbing changes, not
# real feature work.
FROM eclipse-temurin:17-jre-alpine

RUN wget -qO /opt/salsa.jar \
    https://github.com/cosad3s/salsa/releases/download/salsa-1.0.1/salsa-jar-with-dependencies.jar

# Confirmed from SALSA's own bundled logback.xml: it always writes a
# rolling log file to ${application.home:-.}/output/log.<date>.log on
# every run, regardless of --dump/--output - not just when explicitly
# asked to. Left at its own default (CWD), this would try to write
# under wherever the JVM happens to be launched from, which this
# fleet's --read-only containers won't allow. Setting the
# application.home system property to /artifacts instead makes that
# log land as a real, retrievable artifact (it carries the same
# real-time narrative already seen on stdout) rather than fail.
ENTRYPOINT ["java", "-Dapplication.home=/artifacts", "-jar", "/opt/salsa.jar"]
