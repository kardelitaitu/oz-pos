#!/usr/bin/env bash
# scripts/backup-db.sh — SQLite database backup with compression and retention
#
# Copies the active SQLite database using the .backup command (safe, consistent),
# timestamps the output, and compresses with gzip. Automatically prunes backups
# older than the retention period.
#
# Usage:
#   bash scripts/backup-db.sh                           # backup to default dir
#   bash scripts/backup-db.sh /path/to/oz-pos.db        # specific DB file
#   BACKUP_DIR=/backups bash scripts/backup-db.sh       # custom backup dir
#   RETENTION_DAYS=90 bash scripts/backup-db.sh         # keep 90 days
#
# Defaults:
#   DB file: ./oz-pos.db (or OZ_DB_PATH env var)
#   Backup dir: ./backups/
#   Retention: 30 days

set -euo pipefail

DB_FILE="${1:-${OZ_DB_PATH:-oz-pos.db}}"
BACKUP_DIR="${BACKUP_DIR:-backups}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"

# Ensure DB exists
if [ ! -f "$DB_FILE" ]; then
  echo "backup-db: ERROR — database not found: $DB_FILE"
  exit 1
fi

# Create backup directory
mkdir -p "$BACKUP_DIR"

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="$BACKUP_DIR/oz-pos-${TIMESTAMP}.db.gz"

echo "backup-db: backing up $DB_FILE → $BACKUP_FILE"

# Use sqlite3 .backup for a consistent snapshot
sqlite3 "$DB_FILE" ".backup '${BACKUP_FILE%.gz}'"

# Compress
gzip -f "${BACKUP_FILE%.gz}"

SIZE=$(du -h "$BACKUP_FILE" | cut -f1)
echo "backup-db: done — $BACKUP_FILE ($SIZE)"

# Prune old backups
echo "backup-db: pruning backups older than $RETENTION_DAYS days..."
DELETED=$(find "$BACKUP_DIR" -name "oz-pos-*.db.gz" -mtime +"$RETENTION_DAYS" -delete -print | wc -l)
echo "backup-db: removed $DELETED old backup(s)"

# List remaining backups
COUNT=$(find "$BACKUP_DIR" -name "oz-pos-*.db.gz" | wc -l)
echo "backup-db: $COUNT backup(s) retained"
