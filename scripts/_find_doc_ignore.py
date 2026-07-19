"""Find all ```ignore blocks in Rust files with context."""
import os

files_to_check = [
    'modules/crm/src/lib.rs',
    'modules/inventory/src/lib.rs',
    'modules/sales/src/lib.rs',
    'modules/settings/src/lib.rs',
    'modules/staff/src/lib.rs',
    'modules/tax/src/lib.rs',
    'crates/oz-api/src/lib.rs',
    'crates/oz-lua/src/bridge.rs',
    'crates/oz-payment/src/drivers/mock.rs',
    'crates/oz-payment/src/drivers/qris.rs',
    'crates/oz-payment/src/drivers/square.rs',
    'crates/oz-payment/src/drivers/stripe.rs',
    'crates/oz-plugin/src/lib.rs',
    'crates/oz-security/src/lib.rs',
    'platform/core/src/database/pool.rs',
    'platform/kernel/src/event_bus.rs',
    'platform/kernel/src/kernel.rs',
    'platform/kernel/src/lib.rs',
    'platform/startup/src/lib.rs',
    'platform/sync/src/lib.rs',
    'apps/desktop-client/src/commands/authz.rs',
    'apps/desktop-client/src/lib.rs',
]

MARKER = '```ignore'

for filepath in files_to_check:
    if not os.path.exists(filepath):
        print(f"SKIP: {filepath} (not found)")
        continue
    with open(filepath, 'r', encoding='utf-8', errors='replace') as f:
        lines = f.readlines()

    for i, line in enumerate(lines, 1):
        if MARKER in line:
            start = max(0, i-2)
            end = min(len(lines), i+5)
            print(f"=== {filepath}:{i} ===")
            for j in range(start, end):
                marker = ">>>>>" if j+1 == i else "     "
                print(f"{marker} {lines[j].rstrip()}")
            print()
