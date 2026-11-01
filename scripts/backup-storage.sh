#!/usr/bin/env bash
set -euo pipefail

STORAGE_DIR="${STORAGE_DIR:-/opt/reader/storage}"
BACKUP_DIR="${BACKUP_DIR:-/opt/reader/backups}"
RETENTION_DAYS="${BACKUP_RETENTION_DAYS:-14}"

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

require_cmd sqlite3
require_cmd tar
require_cmd find

if [[ ! -d "$STORAGE_DIR" ]]; then
  echo "Storage directory does not exist: $STORAGE_DIR" >&2
  exit 1
fi
if [[ ! "$RETENTION_DAYS" =~ ^[0-9]+$ ]]; then
  echo "BACKUP_RETENTION_DAYS must be a non-negative integer" >&2
  exit 1
fi

mkdir -p "$BACKUP_DIR"
chmod 700 "$BACKUP_DIR"

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
archive="$BACKUP_DIR/reader-storage-$timestamp.tar.gz"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT

cp -a "$STORAGE_DIR/." "$stage/"
rm -f "$stage/reader.db" "$stage/reader.db-wal" "$stage/reader.db-shm"

database="$STORAGE_DIR/reader.db"
if [[ -f "$database" ]]; then
  sqlite3 "$database" ".backup '$stage/reader.db'"
else
  echo "Warning: database not found, backing up storage files only: $database" >&2
fi

tar -C "$stage" -czf "$archive" .
chmod 600 "$archive"
find "$BACKUP_DIR" -maxdepth 1 -type f -name 'reader-storage-*.tar.gz' \
  -mtime "+$RETENTION_DAYS" -delete

echo "$archive"
