---
title: Inventaris & Gudang
description: Pantau stok lintas gudang dengan riwayat pergerakan.
category: guides
order: 5
updated: "2026-08-16"
---

## Level stok

Stok dilacak per produk per lokasi (gudang atau register). Transaksi mengurangi
stok secara otomatis, dan setiap register melayani dari lokasi yang
ditetapkan. Pemilih lokasi mengganti tampilan saat ini, sehingga level selalu
terbaca sesuai konteks.

## Penyesuaian

Penyesuaian stok berjalan dalam dua langkah: pilih produk, lalu pilih alasan —
**Isi ulang** (pengiriman pemasok), **Koreksi stok opname**, **Retur
pelanggan**, **Rusak / kedaluwarsa**, **Penghapusan / kedaluwarsa**,
**Transfer ke lokasi lain**, atau alasan kustom — dan masukkan perubahannya.
Setiap penyesuaian menulis entri buku besar pergerakan, sehingga setiap
perubahan dapat ditelusuri kembali ke siapa, kapan, dan mengapa.

## Stok opname

Stok opname merekonsiliasi sistem dengan jumlah fisik di rak. Mulai **shift
stok** (misalnya `Night shift count`), hitung, dan koreksi tercatat terhadap
shift tersebut. Opname terdaftar dengan filter status, dan masing-masing
membuka tampilan detail dengan riwayatnya, sehingga selisih yang ditemukan
belakangan tetap dapat dijelaskan.

## Batas stok dan peringatan

Peringatan stok rendah menandai produk di bawah ambang batasnya. Batas
dikonfigurasi per lokasi, dengan fallback **Global (Semua Lokasi)** untuk
produk tanpa pengaturan khusus lokasi, dan setiap batas dapat diaktifkan atau
dinonaktifkan secara terpisah.

## Transfer dan transit

Stok berpindah antar lokasi sebagai transfer yang tercatat. Item dalam transit
diaudit dengan sumber, tujuan, jumlah, dan waktu kirimnya; transit yang
terlambat ditandai agar tidak ada yang hilang di antara rak. Transfer yang
keliru dapat **dibalik**, mengembalikan stok ke lokasi asalnya.

## Pesanan pembelian

Pengisian ulang melalui pemasok melewati pesanan pembelian: kelola pemasok,
buat pesanan dengan pemasok dan tanggal pesanan, lalu **Terima** saat
pengiriman tiba — jumlah yang diterima masuk ke stok secara otomatis.

## Laporan dan buku besar pergerakan

**Laporan Stok** menampilkan stok, batas, harga satuan dan biaya, margin,
serta nilai stok per produk, dan dapat dicetak atau diekspor sebagai CSV.
**Log Transaksi Stok** menampilkan setiap pergerakan — transfer, stok opname,
dan penyesuaian manual — sebagai satu buku besar dari mana stok berasal dan
ke mana perginya.
