terminal-management-title = Manajemen Terminal
terminal-management-loading = Memuat terminal…
terminal-management-empty = Belum ada terminal terdaftar. Daftarkan terminal pertama untuk memulai.
terminal-management-error = Gagal memuat terminal. Silakan coba lagi.
terminal-management-retry = Coba Lagi
terminal-register = Daftarkan Terminal
terminal-register-title = Daftarkan Terminal Baru
terminal-edit-title = Ubah Terminal
terminal-delete-title = Hapus Terminal
terminal-delete-confirm = Apakah Anda yakin ingin menghapus terminal "{ $name }"? Tindakan ini tidak dapat dibatalkan.
terminal-name = Nama
terminal-name-label = Nama terminal
terminal-name-placeholder =
    .placeholder = mis. Kasir Depan
terminal-device-id = ID Perangkat
terminal-device-id-label = ID perangkat
terminal-device-id-placeholder =
    .placeholder = mis. nama host atau alamat MAC
terminal-secret = Rahasia Bersama
terminal-secret-label = Rahasia bersama opsional untuk otentikasi sinkronisasi
terminal-metadata = Metadata
terminal-metadata-label = Metadata JSON opsional
terminal-is-active = Aktif
terminal-is-inactive = Tidak Aktif
terminal-status = Status
terminal-last-seen = Terakhir Terlihat
terminal-created = Dibuat
terminal-never = Tidak Pernah
terminal-cancel = Batal
terminal-save = Simpan
terminal-delete = Hapus
terminal-register-action = Daftarkan
terminal-edit-action = Ubah
terminal-delete-action = Hapus
terminal-register-success = Terminal "{ $name }" berhasil didaftarkan.
terminal-update-success = Terminal "{ $name }" berhasil diperbarui.
terminal-delete-success = Terminal berhasil dihapus.
terminal-name-required = Nama wajib diisi.
terminal-device-id-required = ID Perangkat wajib diisi.
terminal-error-load = Gagal memuat terminal
terminal-error-overrides-load = Gagal memuat penimpaan fitur
terminal-error-override-update = Gagal memperbarui penimpaan fitur
terminal-error-override-reset = Gagal mengatur ulang penimpaan
terminal-error-save = Gagal menyimpan terminal
terminal-field-name-aria = Nama terminal
terminal-field-device-id-aria =
    .aria-label = ID perangkat
terminal-field-secret-aria =
    .aria-label = Rahasia bersama
terminal-field-metadata-aria =
    .aria-label = Metadata JSON
terminal-modal-close =
    .aria-label = Tutup
terminal-feature-overrides = Penimpaan Fitur
terminal-loading-overrides = Memuat penimpaan…
terminal-overridden = ditimpa
terminal-override-aria =
    .aria-label = Timpa { $feature }
terminal-reset-overrides = Setel ulang semua penimpaan
terminal-delete-aria =
    .aria-label = Hapus terminal
terminal-col-actions =
    .aria-label = Tindakan
terminal-table-label = Terminal

# Terminal Status Panel
terminal-status-title = Status Terminal
terminal-status-online-count = { $online } / { $total } daring
terminal-status-empty = Belum ada terminal terdaftar.
terminal-status-list-aria = Status terminal
terminal-status-online = Daring
terminal-status-offline = Luring
terminal-status-never = Tidak Pernah
terminal-status-just-now = Baru saja
terminal-status-minutes-ago = { $n }m lalu
terminal-status-hours-ago = { $n }j lalu
terminal-status-error-load = Gagal memuat terminal

# Device binding (ADR #4 Phase 3)
terminal-binding-title = Pengikatan Perangkat
terminal-binding-bound-store = Terikat ke toko:
terminal-binding-signature = Tanda tangan
terminal-binding-valid = Valid
terminal-binding-invalid = Tidak Valid / Dirusak
terminal-binding-store-label = Toko
terminal-binding-instance-label = Instance Ruang Kerja
# Conjunction following middot in binding info paragraph (lowercase).
terminal-binding-instance-conjunction = instance:
terminal-binding-select-store = -- Pilih toko --
terminal-binding-select-instance = -- Pilih instance --
terminal-binding-primary = (Utama)
terminal-binding-update = Perbarui Pengikatan
terminal-binding-bind = Ikat Terminal
terminal-binding-clear = Hapus Pengikatan
terminal-binding-error-load = Gagal memuat pengikatan perangkat
terminal-binding-error-save = Gagal menyimpan pengikatan perangkat
terminal-binding-error-clear = Gagal menghapus pengikatan perangkat

# Feature override counts
terminal-overrides-count = { $count ->
    [one] { $count } penimpaan
   *[other] { $count } penimpaan
}

# ── Promotions ──
