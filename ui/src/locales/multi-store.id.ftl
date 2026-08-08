multi-store-dashboard-title = Dasbor Multi-Toko
multi-store-dashboard-loading = Memuat dasbor…
multi-store-stat-total-stores = Total Toko
multi-store-stat-active-terminals = Terminal Aktif
multi-store-stat-online-terminals = Terminal Daring
multi-store-stat-total-terminals = Total Terminal
multi-store-section-stores = Toko
multi-store-badge-primary = Utama
multi-store-btn-set-primary = Jadikan Utama
multi-store-btn-set-primary-label = Jadikan { $name } sebagai toko utama
multi-store-btn-delete = Hapus
multi-store-btn-delete-label = Hapus { $name }
multi-store-label-address = Alamat
multi-store-label-tax-id = NPWP
multi-store-label-currency = Mata Uang
multi-store-label-timezone = Zona Waktu
multi-store-label-terminals = Terminal
multi-store-section-stores-overview = Ikhtisar toko
multi-store-section-terminal-status = Status terminal
multi-store-error-load = Gagal memuat data

# ── Topology Editor ──

topology-builder-title = Builder Topologi Visual Toko & Workspace

topology-sim-start = Uji Simulasi Pesanan
topology-sim-stop = Hentikan Simulasi
topology-preset-retail = Preset Ritel
topology-preset-restaurant = Preset Resto & KDS
topology-apply-changes = Terapkan Perubahan Topologi
topology-palette-title = Alat Palet
topology-palette-desc = Seret atau klik untuk menambahkan node topologi:
topology-tool-store = + Node Toko
topology-tool-store-desc = Profil Cabang Toko
topology-tool-workspace = + Node Workspace
topology-tool-workspace-desc = Instansi POS / Register
topology-tool-warehouse = + Node Gudang
topology-tool-warehouse-desc = Lokasi Penyimpanan
topology-tool-hardware = + Node Perangkat Keras
topology-tool-hardware-desc = Printer / Periferal KDS
topology-lock-pro = Pro
topology-delete-selected = Hapus Elemen Terpilih
topology-undo = Undo (Ctrl+Z)
topology-redo = Redo (Ctrl+Y)
topology-zoom = Zoom: { $zoom }%
topology-fit-all = Sesuaikan Semua
topology-reset-view = Atur Ulang Tampilan
topology-confirm-delete-node-title = Hapus Node
topology-confirm-delete-wire-title = Hapus Koneksi
topology-confirm-delete-node-msg = Node ini memiliki koneksi. Menghapusnya akan menghapus semua koneksi juga. Tindakan ini tidak dapat dibatalkan.
topology-confirm-delete-wire-msg = Hapus koneksi ini? Tindakan ini tidak dapat dibatalkan.
topology-confirm-delete-label = Hapus
topology-confirm-preset-title = Muat Preset
topology-confirm-preset-msg = Memuat preset akan mengganti topologi saat ini. Perubahan yang belum disimpan akan hilang. Anda dapat membatalkan setelah memuat.
topology-confirm-preset-label = Muat Preset
topology-inspector-title = Inspektur Node
topology-inspector-node-name = Nama Node
topology-inspector-subtitle = Subtitle / Lokasi
topology-tier-suffix = { $tier } TIER
topology-toast-multi-warehouse = Beberapa lokasi gudang memerlukan lisensi Pro Tier.
topology-toast-selection-dropped = Elemen yang dipilih tidak ada di preset ini dan dibatalkan pemilihannya.
topology-toast-wire-duplicate = Koneksi sudah ada di antara port ini.
topology-toast-fallback-warehouse = Koneksi fallback multi-gudang memerlukan lisensi Pro Tier.
topology-toast-load-error = Gagal memuat topologi
topology-wire-incompatible = Konektor ini tidak dapat dihubungkan.
topology-canvas-aria-label = Kanvas editor topologi. Gunakan tombol panah untuk menggeser node, Ctrl+Z untuk undo.
topology-new-store = Toko Baru
topology-new-store-subtitle = Cabang
topology-new-workspace = Workspace Baru
topology-new-workspace-subtitle = Register
topology-new-warehouse = Gudang Baru
topology-new-warehouse-subtitle = Penyimpanan
topology-new-hardware = Perangkat Baru
topology-new-hardware-subtitle = Periferal
topology-new-ready = Siap
topology-ws-type-store-pos = POS Ritel
topology-ws-type-restaurant-pos = POS Restoran
topology-ws-type-kds = Kitchen Display (KDS)
topology-ws-type-warehouse = Gudang

# ── Aria labels & tooltips ──
topology-node-drag-hint = Seret untuk memindahkan
topology-wire-toggle-aria = Alihkan arah koneksi

topology-wire-toggle-hint =
    Klik untuk mengganti arah (satu arah → terbalik → dua arah)
topology-wire-flip-hint-connecting =
    Balik arah? Klik tetap menjaga koneksi Anda tetap berjalan.
topology-inspector-close-aria = Tutup inspektur
topology-inspector-hardware-title = Perangkat Keras
topology-ws-type-select-aria = Pilih tipe workspace
# $name (String) — node display name · $port (String) — port position (top/right/bottom/left)
topology-port-aria = Port { $port } { $name }
topology-port-location-out = Lokasi
topology-port-location-in = Lokasi
topology-port-location-out-aria = Port Lokasi { $name }
topology-port-workspace-out = Operasi
topology-port-operation-in = Operasi
topology-port-operation-in-aria = Port Operasi { $name }
topology-port-stock-in = Stok Masuk
topology-port-stock-out = Stok Keluar
topology-port-ticket-in = Tiket Masuk
topology-port-device-out = Perangkat Keluar
topology-port-generic-in = Input
topology-port-generic-out = Output
topology-port-input-only = Konektor input menerima koneksi; pilih konektor output terlebih dahulu.
topology-field-name = Nama
topology-field-name-aria = Edit nama { $name }
topology-field-enabled = Aktif
topology-field-enabled-aria = Ubah status aktif { $name }
topology-port-location-in-aria = Port Lokasi { $name }
topology-validation-unsupported-schema = Versi skema topologi ini tidak didukung.
topology-validation-missing-branch = Tambahkan tepat satu node Branch Location.
topology-validation-multiple-branches = Pertahankan tepat satu node Branch Location dalam grafik ini.
topology-validation-branch-identity = Branch Location ini tidak memiliki identitas profil toko.
topology-validation-duplicate-node = ID node harus unik.
topology-validation-invalid-purpose = Tujuan workspace ini tidak didukung oleh tipe teknisnya.
topology-validation-missing-location = Hubungkan workspace ini ke Branch Location menggunakan Location In.
topology-validation-multiple-location = Workspace hanya dapat memiliki satu koneksi Location In.
topology-validation-invalid-location = Ini bukan koneksi Branch Location ke Workspace yang valid.
topology-validation-duplicate-wire = Koneksi semantik ini duplikat.
topology-validation-unknown-wire-endpoint = Koneksi ini merujuk node yang tidak ada dalam grafik.
topology-workspace-purpose-label = Tujuan
topology-workspace-technical-type-label = Tipe teknis
topology-workspace-purpose-selector-label = Tujuan workspace
topology-workspace-purpose-selector-aria = Pilih tujuan workspace
topology-purpose-general = Umum
topology-purpose-checkout = Kasir
topology-purpose-returns = Pengembalian
topology-purpose-dining-room = Ruang makan
topology-purpose-kitchen-hot-line = Jalur dapur panas
topology-purpose-stock-control = Pengendalian stok
topology-purpose-receiving = Penerimaan
topology-port-guidance-compatible = Target kompatibel
topology-port-guidance-incompatible = Target tidak kompatibel

# ── Toast messages ──
topology-toast-save-error = Gagal menyimpan topologi
topology-toast-no-session = Tidak ada sesi aktif — tidak dapat menyimpan workspace.
topology-toast-saved = Topologi tersimpan: { $detail }.

# ── Canvas HUD (aria-hidden decorative text) ──
topology-branch-selector-label = Cabang
topology-branch-selector-aria = Pilih cabang topologi
topology-branch-add = Tambah Cabang
topology-branch-add-name-placeholder = Nama cabang baru
topology-branch-add-confirm = Buat
topology-branch-add-cancel = Batal
topology-branch-add-error = Gagal membuat cabang
topology-branch-delete = Hapus Cabang
topology-branch-delete-confirm = Hapus { $name }?
topology-branch-delete-confirm-btn = Hapus
topology-branch-delete-error = Gagal menghapus cabang
topology-branch-rename-label = Ubah nama cabang
topology-branch-rename-placeholder = Nama cabang
topology-branch-rename-error = Gagal mengubah nama cabang
topology-workspace-rename-label = Ubah nama workspace
topology-workspace-rename-placeholder = Nama workspace
topology-workspace-rename-error = Gagal mengubah nama workspace
topology-hud-nodes = { $count } { $count ->
    [one] node
   *[other] node
}
topology-hud-wires = { $count } { $count ->
    [one] koneksi
   *[other] koneksi
}

# ── Wire labels ──
topology-wire-label-connected = Terhubung
topology-wire-label-stock-deduct = Potong Stok (P{ $priority })
topology-wire-label-fallback = Cadangan (P{ $priority })

# ── Offline Queue ──
