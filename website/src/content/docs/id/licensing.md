---
title: Lisensi & Paket
description: Paket, paket gratis selamanya, kedaluwarsa, dan masa tenggang.
category: reference
order: 1
updated: "2026-08-17"
---

## Paket

OZ-POS memiliki lima paket: `free`, `plus`, `pro`, `premium`, dan
`enterprise`. Apa yang dibuka setiap paket — toko, register, gudang,
pembayaran QRIS, sinkron cloud, dan skrip — ditampilkan di
[halaman harga](../../pricing/).

| Kapabilitas         | Gratis | Plus | Pro | Premium | Enterprise |
| ------------------- | ------ | ---- | --- | ------- | ---------- |
| Toko                | 1      | 1    | 2   | Tanpa batas | Tanpa batas |
| Register / toko     | 1      | 2    | 5   | Tanpa batas | Tanpa batas |
| Gudang              | 1      | 2    | 3   | Tanpa batas | Tanpa batas |
| Riwayat penjualan   | 30 hari | Tanpa batas | Tanpa batas | Tanpa batas | Tanpa batas |
| Pembayaran QRIS     | Tidak  | ✓    | ✓   | ✓       | ✓         |
| Sinkron cloud       | Tidak  | ✓    | ✓   | ✓       | ✓         |
| Skrip (Lua)         | Tidak  | Tidak | Tidak | ✓     | ✓         |

Paket tahunan = 2 bulan gratis (bayar 10 bulan, dapat 12).

## Paket Gratis

Paket Gratis bersifat **gratis selamanya** — satu toko, satu register, satu
gudang, dan riwayat penjualan 30 hari. Tidak perlu kunci lisensi untuk
memulai: paket Gratis dimulai pada peluncuran pertama, dan Anda dapat naik
paket kapan saja tanpa menginstal ulang. Setelah 30 hari, transaksi yang
lebih lama disembunyikan di balik ajakan naik paket — tidak ada yang
dihapus.

## Membeli dan mengaktifkan

Paket berbayar dibeli di checkout situs web. Pembayaran bersifat
register-first: checkout meminta Anda masuk dengan email (kode sekali pakai
atau kata sandi) sehingga langganan terhubung ke akun Anda. Kunci lisensi dan
tanda terima tiba melalui email, lalu Anda tempel kunci ke
**Pengaturan → Lisensi** untuk mengaktifkan. Lihat
[Aktivasi Lisensi](../activation/) untuk perjalanan lengkap, dan
[halaman harga](../../pricing/) untuk harga terkini.

## Kedaluwarsa dan tenggang

Langganan memiliki tanggal kedaluwarsa dan masa tenggang. Saat langganan
berakhir, aplikasi memasuki masa tenggang dan tetap berfungsi — termasuk
offline — hingga tanggal tenggang, lalu turun ke paket gratis. Tidak ada
yang dihapus; memperbarui akan memulihkan paket Anda.

## Batas perangkat

Setiap paket berbayar mengizinkan sejumlah register teraktivasi, dan perangkat
terikat ke perangkat keras — kunci lisensi mengaktifkan perangkat tertentu,
bukan siapa pun yang memegang kunci. Admin tenant dapat mencabut perangkat
dari jarak jauh, yang membebaskan slot dan menandatangani keluar perangkat.

## Melihat lisensi Anda

**Pengaturan → Lisensi** menampilkan paket Anda, status, tanggal kedaluwarsa,
masa tenggang, maksimal toko dan instance POS, ID tenant, dan tipe ruang
kerja yang diizinkan. Halaman akun di situs web menampilkan hal yang sama
dari browser Anda, lengkap dengan manajemen perangkat. Lihat
[Aktivasi Lisensi](../activation/).
