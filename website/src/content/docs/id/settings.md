---
title: Pengaturan & Data
description: Branding, struk, mata uang, dan data lokal.
category: reference
order: 2
updated: "2026-08-16"
---

## Bilah samping pengaturan

Pengaturan adalah bilah samping berisi layar fokus: **Umum**, **Tampilan**,
**Nota**, **Sinkronisasi Cloud**, **Fitur**, **Data**, **Staf**,
**Terminal**, **Toko**, **Log Audit**, **Antrean Offline**, **Shift**,
**Tarif Pajak**, **Nilai Tukar**, **Promosi**, **Topologi**,
**Laporan Email**, dan **Lisensi**. Sematkan layar yang sering dipakai agar
tetap di bagian atas.

## Pengaturan toko

Nama usaha, mata uang, tata letak struk, dan bawaan perangkat keras
dikonfigurasi di sini dan tersinkron ke setiap register. **Tarif Pajak** dan
**Nilai Tukar** menambahkan tarif yang dipakai kasir dan laporan. Pengaturan
nota mengontrol lebar kertas, tampilan mata uang dan pajak, pembulatan,
footer, serta printer — per ruang kerja, sehingga setiap layar mencetak
dengan caranya sendiri.

## Tampilan & perangkat

**Tampilan** mengatur tema (mode gelap) yang dipakai perangkat saat boot.
Preferensi per perangkat seperti volume suara ada di terminal; lihat
[Terminal](../terminals/) untuk apa yang mengikuti perangkat alih-alih
pengguna.

## Staf & keamanan

Staf masuk dengan PIN atau kata sandi, dan setiap akun memiliki peran — salah
satu dari lima preset (**pemilik**, **admin**, **manajer**, **staf**, atau
**auditor**) — yang menentukan ruang kerja dan tindakan yang diizinkan.
Tindakan sensitif (pengesampingan harga, void, refund) terverifikasi PIN, dan
**Log Audit** menyimpan catatan yang tidak dapat diubah. Lihat
[Peran Pengguna](../user-roles/) untuk matriks lengkapnya, dan
[Shift & Rekonsiliasi](../shifts/) untuk bagaimana jejak yang sama
merekonsiliasi kas.

## Manajemen data

Layar **Data** mengekspor, mengimpor, dan mencadangkan data Anda. Ekspor
adalah wizard: pilih jenis data (produk, kategori, penjualan, pelanggan,
pengguna, pengaturan) dan rentang tanggal, lalu hasilnya ditulis sebagai
file `.ozpkg` terenkripsi. Ekspor tidak pernah menyertakan kata sandi, dan
impor divalidasi sebelum apa pun diganti. Cadangan database lokal adalah
salinan pemulihan bencana — lihat [Mode Offline-First](../offline-mode/)
untuk bagaimana data hidup di perangkat.

## Sinkron, offline & lisensi

**Sinkronisasi Cloud** dan **Antrean Offline** menampilkan status sinkron dan
apa yang menunggu untuk mencapai cloud — lihat [Sinkron Cloud](../cloud-sync/)
dan [Mode Offline-First](../offline-mode/). **Lisensi** menampilkan paket,
kedaluwarsa, masa tenggang, dan batas Anda — lihat
[Lisensi & Paket](../licensing/).
