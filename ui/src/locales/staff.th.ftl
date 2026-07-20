# ui/src/locales/staff.ftl — Staff management

staff-title = [TH] Staff [/TH]
staff-add-button = [TH] Add Staff [/TH]
staff-loading = [TH] Loading staff… [/TH]
staff-empty = [TH] No staff members yet. [/TH]
staff-empty-cta = [TH] Add your first staff member [/TH]
staff-col-name = [TH] Name [/TH]
staff-col-username = [TH] Username [/TH]
staff-col-role = [TH] Role [/TH]
staff-col-status = [TH] Status [/TH]
staff-col-workspace = [TH] Workspace [/TH]
staff-col-actions =
    .aria-label = [TH] Actions [/TH]
staff-status-active = [TH] Active [/TH]
staff-status-inactive = [TH] Inactive [/TH]
staff-edit = [TH] Edit [/TH]
staff-edit-aria =
    .aria-label = [TH] Edit { $name } [/TH]
staff-deactivate = [TH] Deactivate [/TH]
staff-deactivate-aria =
    .aria-label = [TH] Deactivate { $name } [/TH]
staff-restore = [TH] Restore [/TH]
staff-restore-aria =
    .aria-label = [TH] Reactivate { $name } [/TH]
staff-modal-add-aria =
    .aria-label = [TH] Add staff member [/TH]
staff-modal-edit-aria =
    .aria-label = [TH] Edit staff member [/TH]
staff-modal-add-title = [TH] Add Staff Member [/TH]
staff-modal-edit-title = [TH] Edit Staff Member [/TH]
staff-modal-close =
    .aria-label = [TH] Close [/TH]
staff-field-username-label = [TH] Username * [/TH]
staff-username-placeholder =
    .placeholder = [TH] e.g. jane [/TH]
staff-field-name-label = [TH] Display Name * [/TH]
staff-name-placeholder =
    .placeholder = [TH] e.g. Jane Smith [/TH]
staff-field-pin-edit-label = [TH] New PIN (leave blank to keep current) [/TH]
staff-field-pin-label = [TH] PIN * (4+ characters) [/TH]
staff-pin-edit-placeholder =
    .placeholder = [TH] Leave blank to keep current [/TH]
staff-pin-placeholder =
    .placeholder = [TH] Enter PIN [/TH]
staff-field-role-label = [TH] Role * [/TH]
staff-role-select-default = [TH] Select a role… [/TH]
staff-btn-cancel = [TH] Cancel [/TH]
staff-btn-update = [TH] Update [/TH]
staff-btn-create = [TH] Create [/TH]
staff-error-username-required = [TH] Username is required [/TH]
staff-error-display-name-required = [TH] Display name is required [/TH]
staff-error-role-required = [TH] Please select a role [/TH]
staff-error-pin-length = [TH] PIN must be at least 4 characters [/TH]
staff-error-save-failed = [TH] Failed to save staff member [/TH]
staff-table-aria = [TH] Staff members [/TH]
staff-field-username-aria =
    .aria-label = [TH] Username [/TH]
staff-field-name-aria = [TH] Display Name [/TH]
staff-field-pin-aria = [TH] PIN [/TH]
staff-error-generic = [TH] { $message } [/TH]

# ── Toast Notifications ───────────────────────────────────────────────────
staff-toast-created = [TH] { $name } created successfully [/TH]
staff-toast-updated = [TH] { $name } updated successfully [/TH]
staff-toast-deactivated = [TH] { $name } deactivated [/TH]
staff-toast-restored = [TH] { $name } restored [/TH]

# ── Workspace Access ──────────────────────────────────────────────────────
staff-ws-section-label = [TH] Workspace Access [/TH]
staff-ws-role-defaults = [TH] Use role defaults [/TH]
staff-ws-custom = [TH] Custom [/TH]

# ── Staff Login ──────────────────────────────────────────────────────────
staff-login-title = [TH] OZ-POS [/TH]
staff-login-subtitle = [TH] Staff Login [/TH]
staff-login-step-username = [TH] Enter your username [/TH]
staff-login-step-pin = [TH] Enter your PIN [/TH]
staff-login-progress-aria =
    .aria-label = [TH] Login progress [/TH]
staff-login-username-placeholder =
    .placeholder = [TH] Username [/TH]
staff-login-username-aria =
    .aria-label = [TH] Username [/TH]
staff-login-next = [TH] Next [/TH]
staff-login-pin-section-aria =
    .aria-label = [TH] PIN entry — type digits on your keyboard or use the on-screen keypad [/TH]
staff-login-pin-aria =
    .aria-label = [TH] PIN entry: { $length } of { $max } digits [/TH]
staff-login-keypad-aria =
    .aria-label = [TH] Numeric keypad [/TH]
staff-login-clear = [TH] Clear [/TH]
staff-login-clear-aria =
    .aria-label = [TH] Clear [/TH]
staff-login-backspace-aria =
    .aria-label = [TH] Backspace [/TH]
staff-login-digit-aria =
    .aria-label = [TH] { $digit } [/TH]
staff-login-submit = [TH] Login [/TH]
staff-login-submitting = [TH] Logging in… [/TH]
staff-login-verifying = [TH] Verifying... [/TH]
staff-login-error-deactivated = [TH] Account is deactivated [/TH]
staff-login-error-not-found = [TH] User not found [/TH]
staff-login-error-connection = [TH] Could not verify username. Check your connection. [/TH]
staff-login-back = [TH] ← Back [/TH]
staff-login-copyright = [TH] © 2026 OZ-POS. All rights reserved. [/TH]
staff-login-attempts-remaining = [TH] ({ $count } attempt{ $count -> [1] { "" } *{ "s" } } remaining) [/TH]
staff-login-lockout = [TH] Locked out. Try again in { $seconds }s [/TH]

# ── Fast User Switching (ADR #6) ──────────────────────────────────────────

staff-login-close-aria =
    .aria-label = [TH] Close [/TH]
staff-login-next-aria =
    .aria-label = [TH] Next [/TH]

fastpin-switch-user = [TH] Switch User [/TH]
fastpin-active-user = [TH] Active: { $name } [/TH]
fastpin-enter-pin = [TH] Enter PIN for { $user } [/TH]

