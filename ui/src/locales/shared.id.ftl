-app-name = OZ-POS
app-name = { -app-name }

save = Simpan
cancel = Batal
delete = Hapus
edit = Ubah
close = Tutup
search = Cari
filter = Saring
confirm = Konfirmasi
back = Kembali
next = Lanjut
skip = Lewati
done = Selesai
loading = Memuat…
error-occurred = Terjadi kesalahan
retry = Coba lagi
no-results = Tidak ada hasil
print = Cetak
export = Ekspor
download = Unduh

nav-pos = Terminal POS
nav-dashboard = Dasbor
nav-sales = Riwayat Penjualan
nav-products = Produk
nav-categories = Kategori
nav-staff = Staf
nav-customers = Pelanggan
nav-inventory = Stok
nav-general = Umum
nav-settings = Pengaturan
nav-reports = Laporan
nav-design = Sistem Desain
nav-login = Masuk Staf
nav-logout = Keluar
nav-orders = Pesanan

error-boundary-title = Terjadi kesalahan

# Status Bar
status-bar-connected = Backend terhubung
status-bar-disconnected = Backend terputus
status-bar-checking = Memeriksa koneksi backend
status-bar-authenticating = Mengautentikasi…

# POS Cart Line Items
shared-loading = Memuat…
shell-loading = Memuat…
ds-title = Sistem Desain

# Badge
badge-default = Badge
badge-success = Berhasil
badge-warning = Peringatan
badge-danger = Bahaya
badge-info = Info

# Toast
toast-success = Operasi berhasil
toast-error = Terjadi kesalahan
toast-warning = Silakan periksa input Anda
toast-info = Ini adalah pesan informasional
toast-dismiss-aria =
    .aria-label = Tutup notifikasi
toast-notifications-aria =
    .aria-label = Notifikasi
modal-close-aria =
    .aria-label = Tutup dialog

# Empty state
empty-state-title = Belum ada apa pun di sini
empty-state-desc = Mulai dengan menambahkan item pertama Anda
empty-state-cta = Tambah Produk

# Error state
error-state-title = Terjadi kesalahan
error-state-desc = Terjadi kesalahan yang tidak terduga. Silakan coba lagi.
error-state-retry = Coba Lagi

# Theme toggle
theme-toggle-label = Alihkan tema
theme-toggle-aria =
    .aria-label = Beralih ke mode { $mode ->
        [dark] gelap
       *[light] terang
    }

# Language selector
language-selector-label = Bahasa
language-selector-select-aria =
    .aria-label = Pilih bahasa
locale-en = English
locale-id = Bahasa Indonesia

# Permission denied
permission-denied-title = Akses Ditolak
permission-denied-desc = { $action } memerlukan peran { $requiredRole }.
permission-denied-current = Anda masuk sebagai { $displayName } ({ $roleName }).
permission-denied-go-back = Kembali

# Store switcher
store-switcher-select = Pilih Toko
store-switcher-current-aria = Toko saat ini: { $name }. Klik untuk mengganti.
store-switcher-list-aria = Toko
store-switcher-primary = · Utama

# Gateway status
gateway-status-online-aria = { $name } daring
gateway-status-offline-aria = { $name } luring

# Role badge
role-badge-logged-in-aria = Masuk sebagai { $displayName }, { $roleName }
role-badge-logout-aria = Keluar dari { $displayName }
role-badge-logout-title = Keluar

# Update banner
update-banner-title = Pembaruan tersedia
update-banner-new-version = Versi baru
update-banner-install = Pasang
update-banner-installing = Memasang…
update-banner-install-aria =
    .aria-label = Unduh dan pasang pembaruan
update-banner-installing-aria =
    .aria-label = Memasang pembaruan…
update-banner-dismiss-aria =
    .aria-label = Tutup notifikasi pembaruan

# Navigation section labels
nav-section-operations = Operasional
nav-section-sales = Penjualan
nav-section-products = Produk
nav-section-finance = Keuangan
nav-section-customers = Pelanggan
nav-section-reports = Laporan
nav-section-management = Manajemen
nav-section-inventory = Persediaan
nav-section-settings = Pengaturan
nav-section-dev = Pengembang

# Navigation (remaining)
nav-pos-terminal = Terminal POS
nav-kds = KDS
nav-stock-adjust = Penyesuaian Stok
nav-sales-history = Riwayat Penjualan
nav-eod-report = Laporan Akhir Hari
nav-tax-rates = Tarif Pajak
nav-exchange-rates = Nilai Tukar
nav-loyalty = Loyalitas
nav-terminals = Terminal
nav-stores = Toko
nav-features = Fitur
nav-data = Data
nav-audit-log = Log Audit
nav-offline-queue = Antrian Offline
nav-shifts = Shift
nav-bundles = Bundel
nav-dashboard-report = Dasbor
nav-sales-report = Laporan Penjualan
nav-inventory-report = Laporan Stok
nav-design-system = Sistem Desain
nav-kiosk = Kiosk
nav-tables = Meja
nav-promotions = Promosi
nav-stock = Stok
nav-history = Riwayat
nav-section-app = Aplikasi
nav-sidebar-collapse = Tutup sidebar
nav-sidebar-expand = Buka sidebar
nav-switch-workspace = Ganti Ruang Kerja
nav-main-aria =
    .aria-label = Navigasi utama
nav-tablist-aria =
    .aria-label = Tab navigasi

# Audit Log
audit-log-title = Log Audit
audit-log-load-more = Muat Lebih Banyak
audit-log-loading = Memuat…
audit-log-refresh = Segarkan
audit-log-retry = Coba Lagi
audit-log-filter-all = Semua
audit-log-filter-success = Berhasil
audit-log-filter-failure = Gagal
audit-log-loading-text = Memuat log audit…
audit-log-empty-filtered = Tidak ada entri audit yang cocok dengan filter saat ini.
audit-log-empty-none = Belum ada entri audit. Entri muncul saat penjualan selesai, dibatalkan, atau tindakan staf terjadi.
audit-log-col-date = Tanggal
audit-log-col-action = Tindakan
audit-log-col-target = Target
audit-log-col-user = ID Pengguna
audit-log-col-outcome = Hasil
audit-log-col-details = Detail
audit-log-count = { $count } entri
audit-log-table-label = Entri log audit
audit-log-search-placeholder =
    .placeholder = Cari tindakan, target, atau pengguna…
audit-log-search-label =
    .aria-label = Cari log audit
audit-log-filter-label =
    .aria-label = Saring berdasarkan hasil

# Audit action labels
audit-action-sale-void = Batalkan Penjualan
audit-action-sale-complete = Selesaikan Penjualan
audit-action-sale-refund = Pengembalian Dana
audit-action-login = Masuk Staf
audit-action-login-failed = Login Gagal
audit-action-user-create = Staf Dibuat
audit-action-user-update = Staf Diperbarui
audit-action-product-create = Produk Dibuat
audit-action-product-update = Produk Diperbarui
audit-action-product-delete = Produk Dihapus
audit-action-stock-adjust = Stok Disesuaikan
audit-action-setting-change = Pengaturan Diubah
audit-action-system-backup = Cadangan Dibuat
audit-action-system-export = Ekspor Data
audit-action-system-import = Impor Data
audit-action-system-restore = Pulihkan

# ── Setup Wizard ──
spinner-label = Memuat…

# ── Workspace Home ──
workspace-home-fullscreen-aria = Alihkan layar penuh
workspace-home-user-aria = Masuk sebagai { $name }
workspace-home-loading = Memuat ruang kerja…
workspace-home-subtitle = Pilih ruang kerja untuk memulai
workspace-home-empty = Tidak ada ruang kerja tersedia
workspace-home-empty-desc = Anda belum memiliki akses ke ruang kerja apa pun. Hubungi administrator.
workspace-home-logout = Keluar
workspace-home-logout-confirm-title = Keluar?
workspace-home-logout-confirm-desc = Anda akan kembali ke layar masuk. Semua pekerjaan yang belum disimpan akan hilang.
workspace-home-logout-confirm-cancel = Batal
workspace-home-logout-confirm-confirm = Keluar
workspace-home-shortcut-hint = Tekan { $key } untuk membuka
workspace-card-open-aria = Buka ruang kerja { $name }
workspace-card-no-access-aria = { $name } — tidak tersedia untuk peran Anda
workspace-card-no-access-title = Tidak tersedia untuk peran { $role }
workspace-card-no-access-badge = Tidak tersedia
workspace-home-error-title = Galat Koneksi
workspace-home-error-desc = Tidak dapat memuat ruang kerja. Periksa koneksi Anda dan coba lagi.
workspace-home-retry = Coba Lagi
workspace-home-retry-btn = Muat Ulang



# Auth / License Activation
auth-activate-title = Aktifkan Lisensi
auth-activate-subtitle = Masukkan informasi Anda di bawah ini
auth-email-label = Alamat Email
auth-email-placeholder = toko@example.com
auth-phone-label = Nomor Telepon
auth-phone-placeholder = 08123456789
auth-license-label = Kunci Lisensi
auth-license-placeholder = OZ-PRO-XXXX-XXXX-XXXX
auth-activate-button = Aktifkan Lisensi
auth-activating = Mengaktifkan...
auth-activation-success = Lisensi berhasil diaktifkan!
auth-activation-failed = Gagal mengaktifkan lisensi.
auth-activation-error = Terjadi kesalahan saat aktivasi.
auth-validation-required = Kunci lisensi dan Email wajib diisi.
auth-validation-invalid-email = Format email tidak valid.
auth-validation-phone-required = Nomor telepon wajib diisi.
auth-validation-invalid-phone = Format nomor telepon tidak valid. Masukkan minimal 7 digit.
auth-paste = Tempel
auth-version = Versi { $version }
auth-ip-address = Alamat IP : { $ip }
auth-copyright = OZ-POS © { $year } Hak Cipta Dilindungi.
auth-clipboard-error = Kesalahan papan klip: { $message }
auth-error-title = Kesalahan
