#!/usr/bin/env bash
set -euo pipefail

APP_DIR="${APP_DIR:-/opt/reader/app}"
BRANCH="${BRANCH:-main}"
REPO="${REPO:-https://github.com/shuiyind/reader-next.git}"

if [[ ! -d "$APP_DIR/.git" ]]; then
  mkdir -p "$(dirname "$APP_DIR")"
  git clone --branch "$BRANCH" "$REPO" "$APP_DIR"
fi

cd "$APP_DIR"
git fetch origin "$BRANCH"
git checkout "$BRANCH"
git reset --hard "origin/$BRANCH"

mkdir -p /opt/reader/storage /opt/reader/caddy-data /opt/reader/caddy-config /opt/reader/backups
chmod 700 /opt/reader/backups
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
  invite_code="$(openssl rand -hex 8)"
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

docker compose --env-file .env.prod -f deploy/compose.caddy.yml up -d

docker compose --env-file .env.prod -f deploy/compose.caddy.yml ps

echo "Configure backups with scripts/backup-storage.sh and a daily cron/systemd timer."
echo "Back up .env.prod separately with restrictive permissions and never place it in the data archive."
