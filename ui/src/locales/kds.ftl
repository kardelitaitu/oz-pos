# ui/src/locales/kds.ftl — Kitchen Display System

kds-title = Kitchen Display
kds-screen-aria = Kitchen Display System
kds-pending = Pending
kds-preparing = Preparing
kds-ready = Ready
kds-served = Served
kds-cancelled = Cancelled
kds-item-status-pending = Pending
kds-item-status-preparing = Preparing
kds-item-status-ready = Ready
kds-item-status-served = Served
kds-item-status-cancelled = Cancelled
kds-order-number = Order #
kds-items = { $count } items
kds-notes = Notes
kds-tap-to-advance = Tap to advance
kds-tap-to-advance-label = Order { $number }, tap to advance
kds-no-orders = No orders yet
kds-no-orders-filtered = No orders in this status
kds-order-count = { $count } orders
kds-time-ago-now = now
kds-time-ago = { $minutes }m
kds-urgent-badge = URGENT
kds-pull-to-refresh = Pull down to refresh
kds-release-to-refresh = Release to refresh

# Layout switcher
kds-layout-label = Layout
kds-layout-display-label = Display
kds-layout-options-aria = Layout options
kds-layout-popover-aria = KDS layout and display options
kds-layout-order-id = Order ID
kds-layout-table-number = Table Number
kds-layout-kanban = Kanban
kds-layout-focus = Focus
kds-layout-metro = Metro

# Settings panel
kds-settings-aria = KDS settings
kds-settings-sound = Sound
kds-settings-yellow = Yellow at { $min } min
kds-settings-yellow-aria = Yellow escalation threshold in minutes
kds-settings-red = Red at { $min } min
kds-settings-red-aria = Red escalation threshold in minutes
kds-settings-auto-ack = Auto-acknowledge
kds-settings-density = Density
kds-settings-density-comfortable = Comfortable
kds-settings-density-compact = Compact

# ── 3a: Zone switching ──
kds-zone-filter-aria = Filter by kitchen zone
kds-zone-all = All

# ── 2c: Priority/rush flag ──
kds-rush-badge = RUSH

# ── 2b: History/recall view ──
kds-loading = Loading orders…
kds-history-toggle-aria = Toggle order history
kds-history-toggle-title = Order history
kds-history-filter-aria = Filter by status
kds-history-loading = Loading history...
kds-history-error = Failed to load order history
kds-history-empty = No completed orders yet
kds-history-received = Received
kds-history-served = Served

# ── 3f: Ticket editing ──
kds-edit-items-btn = Edit Items
kds-edit-items-btn-aria = Edit ticket items
kds-edit-items-aria = Edit items
kds-edit-count-label = Count
kds-edit-count-aria = Item count
kds-edit-save = Save
kds-edit-save-aria = Save items
kds-edit-cancel = Cancel
kds-edit-cancel-aria = Cancel edit

# ── 2a: Course names (Phase 2) ──
kds-course-appetizer = APPETIZER
kds-course-main = MAIN
kds-course-side = SIDE
kds-course-dessert = DESSERT
kds-course-beverage = BEVERAGE
kds-course-other = OTHER
kds-course-loading = Loading items...
kds-course-modifier-separator =: 

# ── 3b: Offline resilience ──
kds-offline-label = Offline — showing cached orders
kds-offline-queued = { $count } update(s) queued — offline
kds-offline-queued-update = Update queued — will sync when online
# OFF-05: actions that exhausted retries and need operator attention
kds-offline-dead-letter = { $count } update(s) could not be synced after repeated attempts. Tap Retry to re-queue or clear to dismiss.
kds-offline-dead-letter-aria = Failed updates awaiting operator attention
kds-offline-dead-letter-clear-aria = Clear failed updates
# OFF-08: local persistence is unavailable — queued actions are not durable
kds-offline-storage-unavailable = Local offline storage is unavailable. Queued updates will be lost on reload.
kds-offline-retry = Retry
kds-offline-retry-aria = Retry pending updates
kds-offline-dismiss-aria = Dismiss offline banner

# ── 3d: Voice callout ──
kds-order-up-tts = Order
kds-ready-tts = up

# ── 3f: Add items button + product picker (TODO 3f) ──
kds-add-items-btn = Add Items
kds-add-items-btn-aria = Add items to order
kds-picker-title = Add Items to Order
kds-picker-close-aria = Close picker
kds-picker-search-placeholder = Search products...
kds-picker-search-aria = Search products
kds-picker-loading = Loading products...
kds-picker-error = Failed to load products
kds-picker-no-products = No products found
kds-picker-clear-search = Clear search
kds-picker-selected = Selected
kds-picker-picked-empty = Click products to add them
kds-picker-course-aria = Course
kds-picker-qty-decrease = Decrease quantity
kds-picker-qty-increase = Increase quantity
kds-picker-remove-aria = Remove { $name }
kds-picker-cancel = Cancel
kds-picker-add-btn = Add { $count } item(s)
kds-picker-added-label = added

# ── UX audit: keyboard shortcuts + error retry ──
kds-shortcuts-aria = Keyboard shortcuts
kds-shortcuts-label = Keyboard shortcuts
kds-shortcut-select = Select ticket by position
kds-shortcut-advance = Advance selected ticket
kds-shortcut-navigate = Navigate tickets
kds-shortcut-deselect = Deselect / close
kds-error-retry-aria = Retry
kds-error-dismiss-aria = Dismiss
