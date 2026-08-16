---
title: Shift & Rekonsiliasi
description: Tutup shift kasir dengan rapi dan jejak audit lengkap.
category: guides
order: 4
updated: "2026-08-16"
---

## Membuka shift

Kasir membuka shift di register sebelum melayani pelanggan. Dialog **Buka
Shift** menerima **saldo awal** opsional — uang awal di laci, misalnya
`100.00`. Hanya transaksi kasir tersebut yang dihitung dalam shift, dan jam
berjalan menampilkan berapa lama shift telah berjalan — tetap berpatokan pada
waktu pembukaan asli, sehingga restart atau pembaruan aplikasi tidak pernah
meresetnya. Hanya satu shift yang terbuka di satu register pada satu waktu.

## Penarikan tunai

Uang dapat keluar dari laci di tengah shift tanpa menutupnya — misalnya
penyetoran ke brankas. **Catat Penarikan** menerima jumlah dan alasan
(bawaan `safe drop`), dan penarikan dikurangkan dari perkiraan tunai sehingga
rekonsiliasi saat penutupan tetap akurat.

## Menutup dan merekonsiliasi

**Tutup Shift** menerima jumlah tunai yang **dihitung** di laci dan catatan
opsional. Layar menampilkan total yang diharapkan versus yang dihitung dan
menandai **selisih** — diberi label **Lebih** atau **Kurang** — sebelum
register menerima penutupan, sehingga selisih terlihat di kasir daripada di
akhir bulan. Penutupan ditolak selama masih ada transaksi berjalan; selesaikan
atau kosongkan keranjang terlebih dahulu. Ringkasan shift yang ditutup
langsung ditampilkan.

## Riwayat shift dan akhir hari

Layar manajemen shift menampilkan daftar semua shift dengan status, waktu
buka dan tutup, saldo awal dan jumlah dihitung, perkiraan tunai, selisih, dan
penjualan, serta membuka laporan lengkap per shift. **Laporan Akhir Hari**
merangkum shift hari ini: kartu KPI (total pendapatan, rata-rata penjualan,
void, diskon), rekonsiliasi tunai (total awal vs total dihitung vs total
diharapkan, dengan selisih bersih), rincian pembayaran, dan penjualan per jam
— dapat dicetak dan diekspor.

## Riwayat audit

Setiap transaksi, void, refund, penarikan, dan penyesuaian stok dicatat
dengan pengguna dan terminal yang melakukannya, sehingga setiap shift dapat
direkonsiliasi kembali ke jejak audit yang lengkap.
