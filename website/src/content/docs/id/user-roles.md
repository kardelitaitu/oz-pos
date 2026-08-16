---
title: Peran Pengguna
description: Lima preset izin menentukan apa yang bisa dilakukan dan dilihat setiap akun staf.
category: guides
order: 9
updated: "2026-08-16"
---

## Apa itu peran

Setiap akun staf memiliki peran — preset izin yang menentukan apa yang bisa
dilakukan dan dilihat akun tersebut. Peran berasal dari taksonomi tetap lima
preset, yang ditampilkan saat Anda mengelola staf di **Pengaturan → Staf**.

## Lima peran

| Area akses                         | Staf | Manajer | Auditor | Admin | Pemilik |
| ---------------------------------- | ---- | ------- | ------- | ----- | ------- |
| Penjualan & kasir                  | ✓    | ✓       | —       | ✓     | ✓       |
| Void & refund                      | —    | ✓       | —       | ✓     | ✓       |
| Pembayaran (tunai, kartu, setelmen) | ✓   | ✓       | —       | ✓     | ✓       |
| Diskon (terapkan)                  | ✓    | ✓       | —       | ✓     | ✓       |
| Lampirkan pelanggan & loyalitas di kasir | ✓ | ✓ | —    | ✓     | ✓       |
| Shift (buka, tutup)                | ✓    | ✓       | lihat   | ✓     | ✓       |
| Produk & katalog                   | —    | ✓       | baca    | ✓     | ✓       |
| Ubah biaya produk                  | —    | ✓       | —       | ✓     | ✓       |
| Inventaris (sesuaikan, transfer, opname) | — | ✓ | baca | ✓   | ✓       |
| Pelanggan & loyalitas (kelola)     | —    | ✓       | baca    | ✓     | ✓       |
| Promosi (kelola)                   | —    | ✓       | —       | ✓     | ✓       |
| Akun staf (buat, ubah)             | —    | ✓       | baca    | ✓     | ✓       |
| Kelola peran                       | —    | —       | —       | ✓     | ✓       |
| Hapus staf                         | —    | —       | —       | —     | ✓       |
| Pengaturan                         | —    | ✓       | baca    | ✓     | ✓       |
| Laporan & analitik                 | —    | ✓       | lihat   | ✓     | ✓       |
| Log audit                          | —    | ✓       | lihat   | ✓     | ✓       |
| Tampilan Dapur (lihat, perbarui)   | ✓    | ✓       | lihat   | ✓     | ✓       |
| Terminal (daftarkan, ubah, hapus)  | —    | ✓       | —       | ✓     | ✓       |
| Akses ruang kerja                  | sesuai penugasan | ✓ | ✓ | ✓ | ✓ |

Legenda: **✓** akses penuh · **baca** hanya lihat · **sesuai penugasan**
hanya ruang kerja yang ditugaskan ke akun · **—** tidak ada akses.

## Model yang direncanakan

Matriks ini adalah target untuk basis kode:

- **Staf adalah peran operasional kasir.** Ia mempertahankan tindakan di
  register — memproses penjualan, pembayaran, diskon di keranjang,
  melampirkan pelanggan dan loyalitas, membuka dan menutup shift — plus
  ruang kerja yang ditugaskan. Setiap permukaan manajemen (produk,
  inventaris, pelanggan, promosi, staf, pengaturan, laporan, audit,
  terminal) membutuhkan **manajer ke atas**, dan void, refund, serta
  tindakan sensitif harga juga manajer ke atas.
- **Pemilik** disemai dengan wildcard global. **Admin** bersifat global
  kecuali transfer kepemilikan, penagihan, dan tindakan tak dapat
  dibatalkan seperti penghapusan staf. **Auditor** bersifat global dan
  hanya-baca: melihat data operasional dan log audit, tidak pernah
  mengelola, tidak pernah mengekspor, dan tidak pernah melihat kolom profil
  sensitif.
- **Kustom** adalah preset keenam — tanpa izin sendiri; admin memilih setiap
  izin secara manual. Belum ditampilkan di dropdown staf standar.

## Status implementasi

Empat celah dalam rencana telah ditutup:

- **Preset `Staff` kini hanya kasir** (`platform/core/src/rbac.rs`):
  mempertahankan pemrosesan penjualan, pembayaran, diskon di keranjang,
  lampiran pelanggan dan loyalitas, buka/tutup shift, operasi layanan meja,
  KDS, dan perpindahan ruang kerja — dan tidak yang lain. `sales:void`,
  `sales:refund`, `payments:refund`, `products:*`, `staff:*`, `reports:*`,
  `audit:*`, `terminals:*`, `inventory:*`, dan `promotions:*` dihapus,
  dengan tes terkunci yang diperbarui ke model baru.
- **Semua layar manajemen dibatasi eksplisit.** Pelanggan, Riwayat
  Penjualan, dan kedua layar Dasbor kini menyatakan `requiredRole:
  'manager'`, dan pintu `'manager'` tidak lagi menerima Staf di mana pun.
- **Auditor mencapai layar hanya-bacanya.** Routing menghormati
  `requiredPermission` (mencerminkan `has_permission` backend): `audit:view`
  di log audit, `reports:view` / `inventory:view` di layar laporan,
  `products:read`, `customers:view`, `staff:read`, `settings:read`,
  `shifts:view_any`, dan `loyalty:view` di layar manajemen yang sesuai.
- **Analitik selaras.** Layar Analitik kini menyatakan `requiredRole:
  'manager'` dengan `analytics:view` sebagai kunci izin yang otoritatif.
- **Tombol aksi di dalam layar peka-izin.** Pintu tingkat manajemen
  (`isManager`) tidak lagi menerima Staf, sehingga tombol Void, Refund,
  override harga, tandai-diteliti/ekspor audit, dan kartu pengaturan penuh
  disembunyikan untuk Staf alih-alih menampilkan penolakan backend.
  Dev-mock (`ui/src/dev-mock/tauri-api.ts`) menjalankan model lima peran
  yang nyata — Kasir/Dapur yang pensiun sudah hilang di mana pun, termasuk
  lencana peran, ikon, dan pemilih ruang kerja.
