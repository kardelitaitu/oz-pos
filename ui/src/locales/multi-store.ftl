# ui/src/locales/multi-store.ftl — Multi-store dashboard

multi-store-dashboard-title = Multi-Store Dashboard
multi-store-dashboard-loading = Loading dashboard…
multi-store-stat-total-stores = Total Stores
multi-store-stat-active-terminals = Active Terminals
multi-store-stat-online-terminals = Online Terminals
multi-store-stat-total-terminals = Total Terminals
multi-store-section-stores = Stores
multi-store-badge-primary = Primary
multi-store-btn-set-primary = Set as Primary
multi-store-btn-set-primary-label = Set { $name } as primary store
multi-store-btn-delete = Delete
multi-store-btn-delete-label = Delete { $name }
multi-store-label-address = Address
multi-store-label-tax-id = Tax ID
multi-store-label-currency = Currency
multi-store-label-timezone = Timezone
multi-store-label-terminals = Terminals
multi-store-section-stores-overview = Stores overview
multi-store-section-terminal-status = Terminal status
topology-builder-title = Visual Store & Workspace Topology Builder
multi-store-error-load = Failed to load data

# ── Topology Editor ──

topology-sim-start = Test Order Simulation
topology-sim-stop = Stop Simulation
topology-preset-retail = Retail Preset
topology-preset-restaurant = Resto & KDS Preset
topology-apply-changes = Apply Topology Changes
topology-palette-title = Palette Tools
topology-palette-desc = Drag or click to spawn topology nodes:
topology-tool-store = + Store Node
topology-tool-store-desc = Store Branch Profile
topology-tool-workspace = + Workspace Node
topology-tool-workspace-desc = POS / Register Instance
topology-tool-warehouse = + Warehouse Node
topology-tool-warehouse-desc = Storage Location
topology-tool-hardware = + Hardware Node
topology-tool-hardware-desc = Printer / KDS Peripheral
topology-lock-pro = Pro
topology-delete-selected = Delete Selected Element
topology-undo = Undo (Ctrl+Z)
topology-redo = Redo (Ctrl+Y)
topology-zoom = Zoom: { $zoom }%
topology-fit-all = Fit All
topology-reset-view = Reset View
topology-confirm-delete-node-title = Delete Node
topology-confirm-delete-wire-title = Delete Wire
topology-confirm-delete-node-msg = This node has connected wires. Deleting it will remove all its wires too. This action cannot be undone.
topology-confirm-delete-wire-msg = Delete this wire connection? This action cannot be undone.
topology-confirm-delete-label = Delete
topology-confirm-preset-title = Load Preset
topology-confirm-preset-msg = Loading a preset will replace your current topology. Any unsaved changes will be lost. You can undo this action after loading.
topology-confirm-preset-label = Load Preset
topology-inspector-title = Node Inspector
topology-inspector-node-name = Node Name
topology-inspector-subtitle = Subtitle / Location
topology-tier-suffix = { $tier } TIER
topology-toast-multi-warehouse = Multi-Warehouse storage locations require a Pro Tier license.
topology-toast-wire-duplicate = A wire already connects these ports.
topology-toast-fallback-warehouse = Multi-warehouse stock deduction fallback wires require a Pro Tier license.
topology-toast-load-error = Failed to load topology
topology-canvas-aria-label = Topology editor canvas. Use arrow keys to nudge selected nodes, Ctrl+Z to undo.
topology-new-store = New Store
topology-new-store-subtitle = Branch
topology-new-workspace = New Workspace
topology-new-workspace-subtitle = Register
topology-new-warehouse = New Warehouse
topology-new-warehouse-subtitle = Storage
topology-new-hardware = New Hardware
topology-new-hardware-subtitle = Peripheral
topology-new-ready = Ready
topology-ws-type-store-pos = Retail POS
topology-ws-type-restaurant-pos = Restaurant POS
topology-ws-type-kds = Kitchen Display (KDS)
topology-ws-type-warehouse = Warehouse

# ── Aria labels & tooltips ──
topology-node-drag-hint = Drag to move
topology-wire-toggle-aria = Toggle wire direction
topology-inspector-close-aria = Close inspector
topology-ws-type-select-aria = Select workspace type
# $name (String) — node display name · $port (String) — port position (top/right/bottom/left)
topology-port-aria = { $name } { $port } port

# ── Toast messages ──
topology-toast-save-error = Failed to save topology
topology-toast-no-session = No active session — cannot save workspaces.
topology-toast-saved = Topology saved: { $detail }.

# ── Canvas HUD (aria-hidden decorative text) ──
topology-hud-nodes = { $count } { $count ->
    [one] node
   *[other] nodes
}
topology-hud-wires = { $count } { $count ->
    [one] wire
   *[other] wires
}

# ── Wire labels ──
topology-wire-label-connected = Connected
topology-wire-label-stock-deduct = Stock Deduct (P{ $priority })
topology-wire-label-fallback = Fallback (P{ $priority })
