#!/usr/bin/env bash
set -euo pipefail

readonly SERVICE_USER="slickchook"
readonly SERVICE_GROUP="slickchook"
readonly INSTALL_DIR="/opt/slickchook"
readonly STATE_DIR="/var/lib/slickchook"
readonly UNIT_PATH="/etc/systemd/system/slickchook.service"
readonly CONFIG_DIR="/etc/slickchook"
readonly CONFIG_PATH="${CONFIG_DIR}/slickchook.toml"
readonly BACKUP_ROOT="/var/backups/slickchook"
readonly PLUGIN_COMMAND="/usr/local/sbin/slickchook-plugin"
readonly DB_NAME="slickchook_db"
readonly DB_ROLE="slickchook"
readonly DB_ENV="/etc/slickchook/postgres.env"
readonly PLUGIN_SECRETS_ENV="/etc/slickchook/plugin-secrets.env"
# The hostname the TLS certificate is issued for and Caddy's site block is
# keyed to. Connecting clients (e.g. a paired AI-agent instance) must use
# this same hostname, not a bare IP: TLS servers select a certificate by the
# ClientHello's SNI, and clients don't send SNI for literal IP addresses,
# so a certificate lookup by IP alone fails the handshake outright.
readonly CADDY_HOSTNAME="${CADDY_HOSTNAME:-slickchook}"
readonly CADDY_LISTEN_PORT="${CADDY_LISTEN_PORT:-8443}"
readonly CADDY_CERT_DAYS="${CADDY_CERT_DAYS:-365}"
readonly CADDY_TLS_DIR="/etc/caddy/tls"
readonly CADDYFILE="/etc/caddy/Caddyfile"

SERVICE_WAS_ACTIVE=false
INSTALL_COMPLETE=false
BACKUP_DIR=""

die() {
  echo "error: $*" >&2
  exit 1
}

ensure_backup_dir() {
  if [[ -z ${BACKUP_DIR} ]]; then
    BACKUP_DIR="${BACKUP_ROOT}/$(date -u +%Y%m%dT%H%M%SZ)-$$"
    install -d -o root -g root -m 0700 "${BACKUP_DIR}"
    echo "Backing up replaced files to ${BACKUP_DIR}"
  fi
}

restart_previous_service_on_error() {
  if [[ ${INSTALL_COMPLETE} == false && ${SERVICE_WAS_ACTIVE} == true ]]; then
    echo "Installation failed; attempting to restart the previous service." >&2
    systemctl start slickchook.service || true
  fi
}

trap restart_previous_service_on_error EXIT

if [[ ${EUID} -ne 0 ]]; then
  die "run this script as root: sudo $0"
fi
# sudo resets most environment variables by default. To override
# CADDY_HOSTNAME/CADDY_LISTEN_PORT/CADDY_CERT_DAYS, either run this script
# from a root shell directly, or pass
# sudo --preserve-env=CADDY_HOSTNAME,CADDY_LISTEN_PORT,CADDY_CERT_DAYS.

SOURCE_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
SERVER_BINARY="${SOURCE_DIR}/target/release/slickchook"
RUNNER_BINARY="${SOURCE_DIR}/target/release/slickchook-runner"
PLUGIN_COMMAND_SOURCE="${SOURCE_DIR}/scripts/slickchook-plugin"

[[ -x ${SERVER_BINARY} ]] || die "missing ${SERVER_BINARY}; run cargo build --locked --release --bins first"
[[ -x ${RUNNER_BINARY} ]] || die "missing ${RUNNER_BINARY}; run cargo build --locked --release --bins first"
[[ -f ${SOURCE_DIR}/deploy/slickchook.service ]] || die "missing systemd unit template"
[[ -x ${PLUGIN_COMMAND_SOURCE} ]] || die "missing executable plugin installer: ${PLUGIN_COMMAND_SOURCE}"
command -v openssl >/dev/null || die "openssl is required to generate the Caddy TLS certificate"

command -v apt-get >/dev/null || die "this installer requires Ubuntu or another apt-based system"
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install --yes acl iproute2 postgresql postgresql-client

if ! command -v caddy >/dev/null; then
  if ! DEBIAN_FRONTEND=noninteractive apt-get install --yes caddy; then
    # Older Ubuntu releases don't carry caddy in their default repos; fall
    # back to Caddy's official apt repository.
    # https://caddyserver.com/docs/install#debian-ubuntu-raspbian
    DEBIAN_FRONTEND=noninteractive apt-get install --yes debian-keyring debian-archive-keyring apt-transport-https
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \
      | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
    curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \
      > /etc/apt/sources.list.d/caddy-stable.list
    apt-get update
    DEBIAN_FRONTEND=noninteractive apt-get install --yes caddy
  fi
fi

getent group docker >/dev/null || die "the docker group does not exist; install Docker Engine first"
systemctl enable --now docker.service
systemctl enable --now postgresql.service
runuser -u postgres -- psql --command "SELECT 1 FROM pg_database WHERE datname = '${DB_NAME}'" \
  | grep -q 1 || runuser -u postgres -- createdb "${DB_NAME}"
runuser -u postgres -- psql --command "SELECT 1 FROM pg_roles WHERE rolname = '${DB_ROLE}'" \
  | grep -q 1 || runuser -u postgres -- createuser --no-createdb --no-createrole --no-superuser "${DB_ROLE}"
runuser -u postgres -- psql --command "ALTER DATABASE ${DB_NAME} OWNER TO ${DB_ROLE};"
# Use a locally generated hex password so the service can connect over TCP
# without embedding credentials in the repository or unit file.
DB_PASSWORD=$(od -An -N24 -tx1 /dev/urandom | tr -d ' \n')
runuser -u postgres -- psql --command "ALTER ROLE ${DB_ROLE} PASSWORD '${DB_PASSWORD}';"
install -d -o root -g "${SERVICE_GROUP}" -m 0750 /etc/slickchook
printf 'DATABASE_URL=postgresql://%s:%s@127.0.0.1:5432/%s\n' "${DB_ROLE}" "${DB_PASSWORD}" "${DB_NAME}" > "${DB_ENV}"
chown root:"${SERVICE_GROUP}" "${DB_ENV}"
chmod 0640 "${DB_ENV}"

for _ in {1..20}; do
  [[ -S /var/run/docker.sock ]] && break
  sleep 0.25
done
[[ -S /var/run/docker.sock ]] || die "Docker started but /var/run/docker.sock was not created"

SOCKET_GROUP=$(stat -c '%G' /var/run/docker.sock)
[[ ${SOCKET_GROUP} == "docker" ]] || die "/var/run/docker.sock is owned by group ${SOCKET_GROUP}, expected docker"

if [[ -e ${INSTALL_DIR}/slickchook || -e ${INSTALL_DIR}/slickchook-runner || -d ${STATE_DIR} ]] \
  || systemctl cat slickchook.service >/dev/null 2>&1; then
  echo "Existing Slickchook installation detected; performing an in-place update."
else
  echo "No existing Slickchook installation detected; performing a new install."
fi

if systemctl is-active --quiet slickchook.service; then
  SERVICE_WAS_ACTIVE=true
  echo "Stopping the existing slickchook.service before replacing files."
  systemctl stop slickchook.service
fi

if ss -ltnH 'sport = :8080' | grep -q .; then
  die "port 8080 is used by a process not managed by slickchook.service; stop it and run the installer again"
fi

if ! getent group "${SERVICE_GROUP}" >/dev/null; then
  groupadd --system "${SERVICE_GROUP}"
fi

if ! id "${SERVICE_USER}" >/dev/null 2>&1; then
  useradd --system \
    --gid "${SERVICE_GROUP}" \
    --home-dir "${STATE_DIR}" \
    --shell /usr/sbin/nologin \
    "${SERVICE_USER}"
else
  # Normalize accounts created by older installation instructions.
  usermod \
    --gid "${SERVICE_GROUP}" \
    --home "${STATE_DIR}" \
    --shell /usr/sbin/nologin \
    "${SERVICE_USER}"
fi

[[ $(id -gn "${SERVICE_USER}") == "${SERVICE_GROUP}" ]] || die "${SERVICE_USER} has the wrong primary group"
if id -nG "${SERVICE_USER}" | tr ' ' '\n' | grep -qx docker; then
  echo "Removing unsafe legacy docker-group membership from ${SERVICE_USER}."
  gpasswd --delete "${SERVICE_USER}" docker
fi
if id -nG "${SERVICE_USER}" | tr ' ' '\n' | grep -qx docker; then
  die "failed to remove ${SERVICE_USER} from the docker group"
fi

install -d -o root -g root -m 0755 "${INSTALL_DIR}"

if [[ -f ${INSTALL_DIR}/slickchook ]] && ! cmp -s "${SERVER_BINARY}" "${INSTALL_DIR}/slickchook"; then
  ensure_backup_dir
  cp -a "${INSTALL_DIR}/slickchook" "${BACKUP_DIR}/slickchook"
fi
if [[ -f ${INSTALL_DIR}/slickchook-runner ]] && ! cmp -s "${RUNNER_BINARY}" "${INSTALL_DIR}/slickchook-runner"; then
  ensure_backup_dir
  cp -a "${INSTALL_DIR}/slickchook-runner" "${BACKUP_DIR}/slickchook-runner"
fi

install -o root -g root -m 0755 "${SERVER_BINARY}" "${INSTALL_DIR}/slickchook"
install -o root -g docker -m 0750 "${RUNNER_BINARY}" "${INSTALL_DIR}/slickchook-runner"
setfacl -m "u:${SERVICE_USER}:rx,m::rx" "${INSTALL_DIR}/slickchook-runner"

# Apply this last: ownership and ACL operations can clear setgid on some systems.
chmod 0751 "${INSTALL_DIR}/slickchook-runner"
chmod g+s "${INSTALL_DIR}/slickchook-runner"

RUNNER_MODE=$(stat -c '%a' "${INSTALL_DIR}/slickchook-runner")
RUNNER_OWNER=$(stat -c '%U:%G' "${INSTALL_DIR}/slickchook-runner")
[[ ${RUNNER_MODE} == "2751" ]] || die "runner mode is ${RUNNER_MODE}, expected 2751"
[[ ${RUNNER_OWNER} == "root:docker" ]] || die "runner owner is ${RUNNER_OWNER}, expected root:docker"
getfacl -cp "${INSTALL_DIR}/slickchook-runner" | grep -qx "user:${SERVICE_USER}:r-x" \
  || die "runner ACL does not grant ${SERVICE_USER} read/execute access"

if findmnt -T "${INSTALL_DIR}/slickchook-runner" -no OPTIONS | tr ',' '\n' | grep -qx nosuid; then
  die "${INSTALL_DIR} is on a nosuid filesystem; setgid runner execution will not work"
fi

install -d -o "${SERVICE_USER}" -g "${SERVICE_GROUP}" -m 0750 "${STATE_DIR}"
install -d -o root -g "${SERVICE_GROUP}" -m 0750 "${CONFIG_DIR}"
if [[ ! -f ${CONFIG_PATH} ]]; then
  printf '%s\n' 'bind_addr = "127.0.0.1:8080"' > "${CONFIG_PATH}"
  chown root:"${SERVICE_GROUP}" "${CONFIG_PATH}"
  chmod 0640 "${CONFIG_PATH}"
fi
# Created empty and left alone on upgrade: this is where an admin configures
# API keys/credentials a plugin's manifest declares via [[secrets]] (see
# RUNNER.md), one KEY=value per line, matching each plugin's declared
# `env_var` exactly. Loaded by the unit's `EnvironmentFile=-` directive,
# which tolerates the file being empty or (on a fresh install) not yet
# customized.
if [[ ! -f ${PLUGIN_SECRETS_ENV} ]]; then
  : > "${PLUGIN_SECRETS_ENV}"
  chown root:"${SERVICE_GROUP}" "${PLUGIN_SECRETS_ENV}"
  chmod 0640 "${PLUGIN_SECRETS_ENV}"
fi
install -d -o root -g "${SERVICE_GROUP}" -m 0750 "${STATE_DIR}/plugins"
chown -R root:"${SERVICE_GROUP}" "${STATE_DIR}/plugins"
find "${STATE_DIR}/plugins" -type d -exec chmod 0750 {} +
find "${STATE_DIR}/plugins" -type f -exec chmod 0640 {} +

install -d -o "${SERVICE_USER}" -g "${SERVICE_GROUP}" -m 0750 "${STATE_DIR}/artifacts"
# uid 65534 covers the fleet's usual unprivileged plugin containers; uid 0
# covers the few plugins whose manifest sets DockerPolicy.run_as_root
# (e.g. nmap, whose raw-socket features need actual root - see that
# field's doc comment in src/plugin_manager.rs). Without the uid 0 entry,
# a root-mode plugin's artifact-permission-fixup step gets a bare
# "Permission denied" trying to even traverse this directory, since a
# root process without CAP_DAC_OVERRIDE (stripped by --cap-drop ALL) is
# still subject to normal permission bits, confirmed live.
setfacl -m u:65534:rwx,u:0:rwx,m::rwx "${STATE_DIR}/artifacts"
setfacl -m d:u:65534:rwx,d:u:0:rwx,d:m::rwx "${STATE_DIR}/artifacts"

if [[ -f ${UNIT_PATH} ]] && ! cmp -s "${SOURCE_DIR}/deploy/slickchook.service" "${UNIT_PATH}"; then
  ensure_backup_dir
  cp -a "${UNIT_PATH}" "${BACKUP_DIR}/slickchook.service"
fi

if [[ -f ${PLUGIN_COMMAND} ]] && ! cmp -s "${PLUGIN_COMMAND_SOURCE}" "${PLUGIN_COMMAND}"; then
  ensure_backup_dir
  cp -a "${PLUGIN_COMMAND}" "${BACKUP_DIR}/slickchook-plugin"
fi

install -o root -g root -m 0644 "${SOURCE_DIR}/deploy/slickchook.service" "${UNIT_PATH}"
install -o root -g root -m 0755 "${PLUGIN_COMMAND_SOURCE}" "${PLUGIN_COMMAND}"
systemctl daemon-reload
systemctl enable --now slickchook.service
systemctl restart slickchook.service

# Front slickchook with Caddy for TLS. The certificate is generated once and
# left in place on reinstall/update: regenerating it would silently break
# certificate pinning on whatever client (e.g. a paired AI-agent instance)
# was handed its fingerprint at provisioning time. Delete
# ${CADDY_TLS_DIR}/server.{crt,key} manually to force reissuance.
install -d -o root -g root -m 0755 "${CADDY_TLS_DIR}"
if [[ ! -f ${CADDY_TLS_DIR}/server.crt || ! -f ${CADDY_TLS_DIR}/server.key ]]; then
  echo "Generating Caddy TLS certificate for '${CADDY_HOSTNAME}' (valid ${CADDY_CERT_DAYS} days)."
  openssl req -new -x509 -days "${CADDY_CERT_DAYS}" -nodes \
    -out "${CADDY_TLS_DIR}/server.crt" -keyout "${CADDY_TLS_DIR}/server.key" \
    -subj "/CN=${CADDY_HOSTNAME}" \
    -addext "subjectAltName=DNS:${CADDY_HOSTNAME}" \
    -addext "basicConstraints=critical,CA:FALSE" \
    -addext "keyUsage=critical,digitalSignature,keyEncipherment" \
    -addext "extendedKeyUsage=serverAuth"
else
  echo "Existing Caddy TLS certificate found at ${CADDY_TLS_DIR}/server.crt; leaving it in place."
fi
chown root:caddy "${CADDY_TLS_DIR}/server.key"
chmod 0640 "${CADDY_TLS_DIR}/server.key"
chmod 0644 "${CADDY_TLS_DIR}/server.crt"

if [[ -f ${CADDYFILE} ]] && ! grep -qF "${CADDY_HOSTNAME}:${CADDY_LISTEN_PORT}" "${CADDYFILE}" 2>/dev/null; then
  ensure_backup_dir
  cp -a "${CADDYFILE}" "${BACKUP_DIR}/Caddyfile"
fi
# reverse_proxy targets slickchook's bind_addr default (127.0.0.1:8080, set
# above when CONFIG_PATH is first created). If bind_addr is later customized
# in ${CONFIG_PATH}, update this target to match.
cat > "${CADDYFILE}" <<EOF
{
  auto_https off
}
${CADDY_HOSTNAME}:${CADDY_LISTEN_PORT} {
  tls ${CADDY_TLS_DIR}/server.crt ${CADDY_TLS_DIR}/server.key
  reverse_proxy 127.0.0.1:8080
}
EOF
systemctl enable --now caddy.service
systemctl reload caddy.service || systemctl restart caddy.service

runuser -u "${SERVICE_USER}" -- "${INSTALL_DIR}/slickchook-runner" --help >/dev/null 2>&1
systemctl is-active --quiet slickchook.service || die "slickchook.service did not start"

INSTALL_COMPLETE=true
trap - EXIT

echo "Slickchook installed successfully."
stat -c '%A %a %U:%G %n' "${INSTALL_DIR}/slickchook-runner"
echo "MCP endpoint (local, plaintext): http://127.0.0.1:8080/sse"
echo "MCP endpoint (via Caddy, TLS):   https://${CADDY_HOSTNAME}:${CADDY_LISTEN_PORT}/sse"
echo "Connecting clients must resolve '${CADDY_HOSTNAME}' to this host (e.g. via"
echo "/etc/hosts or DNS) and trust exactly this certificate -- not a shared CA"
echo "or the system trust store -- pinned at provisioning time, not on first use:"
echo "  ${CADDY_TLS_DIR}/server.crt"
openssl x509 -in "${CADDY_TLS_DIR}/server.crt" -noout -fingerprint -sha256
echo "Plugin manager: ${PLUGIN_COMMAND}"
echo "Logs: journalctl -u slickchook.service -n 100 --no-pager"
echo "Caddy logs: journalctl -u caddy.service -n 100 --no-pager"
if [[ -n ${BACKUP_DIR} ]]; then
  echo "Backup of replaced files: ${BACKUP_DIR}"
fi
