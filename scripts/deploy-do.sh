#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${APP_DIR:-/opt/reader/app}"
BRANCH="${BRANCH:-main}"
REPO="${REPO:-https://github.com/shuiyind/reader-next.git}"
TLS_MODE="${TLS_MODE:-auto}"

if [[ ! -d "$APP_DIR/.git" ]]; then
  mkdir -p "$(dirname "$APP_DIR")"
  git clone --branch "$BRANCH" "$REPO" "$APP_DIR"
fi

cd "$APP_DIR"
git fetch origin "$BRANCH"
git checkout "$BRANCH"
git reset --hard "origin/$BRANCH"

mkdir -p /opt/reader/storage /opt/reader/caddy-data /opt/reader/caddy-config /opt/reader/backups
chown -R 10001:10001 /opt/reader/storage
chmod 700 /opt/reader/storage /opt/reader/backups
if [[ -z "${READER_DOMAIN:-}" ]]; then
  echo "READER_DOMAIN is required for the Caddy HTTPS deployment." >&2
  echo "Example: READER_DOMAIN=reader.example.com $0" >&2
  exit 1
fi
if [[ ! "$READER_DOMAIN" =~ ^[A-Za-z0-9.-]+$ || "$READER_DOMAIN" == "reader.example.com" || "$READER_DOMAIN" == "localhost" ]]; then
  echo "READER_DOMAIN must be the public hostname used for HTTPS." >&2
  exit 1
fi

if [[ ! -f .env.prod ]]; then
  cp deploy/env.prod.example .env.prod
  secure_key="$(openssl rand -hex 32)"
  invite_code="$(openssl rand -hex 16)"
  sed -i "s/^SECURE_KEY=.*/SECURE_KEY=${secure_key}/" .env.prod
  sed -i "s/^INVITE_CODE=.*/INVITE_CODE=${invite_code}/" .env.prod
  echo "Created .env.prod with generated SECURE_KEY and INVITE_CODE."
fi

if grep -q '^READER_DOMAIN=' .env.prod; then
  sed -i "s/^READER_DOMAIN=.*/READER_DOMAIN=${READER_DOMAIN}/" .env.prod
else
  printf 'READER_DOMAIN=%s\n' "$READER_DOMAIN" >> .env.prod
fi
chmod 600 .env.prod

if grep -Eq '^SECURE_KEY=(change-me.*|$)' .env.prod || grep -Eq '^INVITE_CODE=(change-me.*|$)' .env.prod; then
  echo ".env.prod still contains empty or placeholder secrets." >&2
  exit 1
fi

case "$TLS_MODE" in
  auto)
    COMPOSE_FILE="deploy/compose.caddy.yml"
    ;;
  single-port)
    COMPOSE_FILE="deploy/compose.caddy.single-port.yml"
    CERT_DIR="${READER_CERT_DIR:-/opt/reader/certs}"
    mkdir -p "$CERT_DIR"
    chown -R 1000:1000 "$CERT_DIR"
    chmod 700 "$CERT_DIR"
    if [[ ! -f "$CERT_DIR/fullchain.pem" || ! -f "$CERT_DIR/privkey.pem" ]]; then
      echo "single-port mode requires $CERT_DIR/fullchain.pem and $CERT_DIR/privkey.pem." >&2
      exit 1
    fi
    if grep -q '^READER_CERT_DIR=' .env.prod; then
      sed -i "s|^READER_CERT_DIR=.*|READER_CERT_DIR=${CERT_DIR}|" .env.prod
    else
      printf 'READER_CERT_DIR=%s\n' "$CERT_DIR" >> .env.prod
    fi
    ;;
  *)
    echo "TLS_MODE must be 'auto' or 'single-port'." >&2
    exit 1
    ;;
esac

docker compose --env-file .env.prod -f "$COMPOSE_FILE" up -d

docker compose --env-file .env.prod -f "$COMPOSE_FILE" ps

echo "TLS_MODE=${TLS_MODE}, compose file=${COMPOSE_FILE}."
echo "Configure backups with scripts/backup-storage.sh and a daily cron/systemd timer."
echo "Back up .env.prod and TLS private keys separately with restrictive permissions and never place secrets in the data archive."
