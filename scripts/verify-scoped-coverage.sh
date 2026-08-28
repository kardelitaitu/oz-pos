#!/usr/bin/env bash
# scripts/verify-scoped-coverage.sh — H-1/H-2 regression guard.
#
# Ensures every registered Tauri command in the desktop client either:
#  1. Has a corresponding _scoped variant, OR
#  2. Is in the allowlist of commands that are intentionally unscoped.
#
# Usage:  bash scripts/verify-scoped-coverage.sh
#         (run from the workspace root)

set -euo pipefail

cd "$(dirname "$0")/.."

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

violations=0

# Allowlist: commands that are intentionally unscoped
ALLOWLIST="staff_login|staff_check_username|has_users|bootstrap_owner|create_session|destroy_session|session_keepalive|verify_pin|activate_license|check_license_status|get_license_status|get_machine_id|get_hardware_fingerprint|renew_license|pause_subscription|resume_subscription|test_auth_connection|ping|version|get_device_id|get_local_ip|resolve_boot_store|get_subscription_capabilities|complete_setup|dismiss_setup_wizard|get_setup_status|get_enabled_features|load_topology|can_save_topology|apply_topology_diff|recover_pending_topology_apply_at_startup|export_data|import_preview|import_data|create_backup|get_backup_status|edc_terminal_status|edc_sale|edc_refund|edc_void|send_test_report|save_report_schedule|get_report_schedule|list_all_features|set_feature|set_features_bulk|get_key_rotation_info|rotate_encryption_key|currency_info|pick_logo_file|settings_changed_sink|create_inventory_location|create_inventory_transaction|deactivate_inventory_location|delete_stock_threshold|end_inventory_shift|finalize_sale|get_active_inventory_shift|get_inventory_transaction|get_stock_thresholds|get_workspace_inventory_locations|list_inventory_locations|list_inventory_shifts|list_inventory_transactions|list_inventory_transactions_for_shift|set_stock_threshold|set_workspace_inventory_locations|start_inventory_shift|update_inventory_location|void_pending_sale|list_warehouse_products_at_location"

# Get all _scoped function names
scoped_funcs=$(grep -roh "pub async fn [a-z_]*_scoped" apps/desktop-client/src/commands --include="*.rs" 2>/dev/null | sed 's/pub async fn //' | sort -u)

# Get all registered unscoped commands (not _scoped themselves)
registered_unscoped=$(grep -oE 'commands::[a-z_]+::[a-z_]+,' apps/desktop-client/src/lib.rs | sed 's/,$//' | grep -v '_scoped$' | sort -u)

echo "=== Scoped Coverage Check ==="
echo ""

while IFS= read -r cmd; do
    fn_name=$(echo "$cmd" | awk -F:: '{print $NF}')

    # Check if it has a _scoped counterpart
    if echo "$scoped_funcs" | grep -q "^${fn_name}_scoped$"; then
        continue
    fi

    # Check if it's in the allowlist
    if echo "$fn_name" | grep -qE "^($ALLOWLIST)$"; then
        continue
    fi

    echo -e "${RED}VIOLATION${NC}: $cmd has no _scoped variant"
    violations=$((violations + 1))
done <<< "$registered_unscoped"

echo ""

if [ "$violations" -gt 0 ]; then
    echo -e "${RED}FAIL: $violations command(s) without _scoped variant or allowlist entry${NC}"
    exit 1
else
    echo -e "${GREEN}PASS: all registered commands covered${NC}"
    exit 0
fi
