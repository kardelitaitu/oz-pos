#!/usr/bin/env bash
# scripts/restore-db.sh — Restore a SQLite database from backup
#
# Takes a backup file path, verifies integrity with .integrity_check,
# replaces the active database, and validates with a smoke query.
#
# Usage:
#   bash scripts/restore-db.sh backups/oz-pos-20260720-120000.db.gz
#   bash scripts/restore-db.sh backups/oz-pos-20260720-120000.db.gz /path/to/oz-pos.db
#   RESTORE_NO_CONFIRM=1 bash scripts/restore-db.sh ...   # skip prompt
#
# Safety: creates a pre-restore backup of the current DB before replacing it.

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: bash scripts/restore-db.sh <backup-file> [target-db]"
  echo "  backup-file: path to .db or .db.gz backup"
  echo "  target-db:   path to replace (default: oz-pos.db)"
  exit 1
fi

BACKUP_FILE="$1"
TARGET_DB="${2:-${OZ_DB_PATH:-oz-pos.db}}"

# Verify backup exists
if [ ! -f "$BACKUP_FILE" ]; then
  echo "restore-db: ERROR — backup file not found: $BACKUP_FILE"
  exit 1
fi

# Decompress if needed
RESTORE_DB="$BACKUP_FILE"
CLEANUP_DB=""
if [[ "$BACKUP_FILE" == *.gz ]]; then
  echo "restore-db: decompressing $BACKUP_FILE..."
  RESTORE_DB="${BACKUP_FILE%.gz}"
  gunzip -c "$BACKUP_FILE" > "$RESTORE_DB"
  CLEANUP_DB="$RESTORE_DB"
else
  RESTORE_DB="$BACKUP_FILE"
fi

# Integrity check
cleanup_on_exit() {
  if [ -n "$CLEANUP_DB" ] && [ -f "$CLEANUP_DB" ] && [ "$CLEANUP_DB" != "$TARGET_DB" ]; then
    rm -f "$CLEANUP_DB"
  fi
}
trap cleanup_on_exit EXIT

echo "restore-db: verifying backup integrity..."
if ! sqlite3 "$RESTORE_DB" "PRAGMA integrity_check;" 2>&1 | grep -q "ok"; then
  echo "restore-db: ERROR — backup integrity check FAILED"
  exit 1
fi
echo "restore-db: integrity check PASSED"

# Smoke query — count key tables
TABLE_COUNT=$(sqlite3 "$RESTORE_DB" "SELECT COUNT(*) FROM sqlite_master WHERE type='table';")
echo "restore-db: backup contains $TABLE_COUNT tables"

# Confirm unless RESTORE_NO_CONFIRM is set
if [ "${RESTORE_NO_CONFIRM:-}" != "1" ]; then
  echo ""
  echo "WARNING: This will replace $TARGET_DB with $BACKUP_FILE"
  echo "  Current DB will be backed up to ${TARGET_DB}.pre-restore"
  read -r -p "Proceed? [y/N] " CONFIRM
  if [ "$CONFIRM" != "y" ] && [ "$CONFIRM" != "Y" ]; then
    echo "restore-db: aborted"
    rm -f "${RESTORE_DB}" 2>/dev/null || true
    exit 0
  fi
fi

# Create pre-restore safety backup
if [ -f "$TARGET_DB" ]; then
  cp "$TARGET_DB" "${TARGET_DB}.pre-restore"
  echo "restore-db: pre-restore backup saved to ${TARGET_DB}.pre-restore"
fi

# Replace the active database
mv "$RESTORE_DB" "$TARGET_DB"
echo "restore-db: restored $TARGET_DB from $BACKUP_FILE"

# Final validation
if sqlite3 "$TARGET_DB" "SELECT 1;" > /dev/null 2>&1; then
  echo "restore-db: final smoke query PASSED — restore complete"
else
  echo "restore-db: WARNING — final smoke query FAILED. Restore ${TARGET_DB}.pre-restore if needed."
fi
