---
title: Mode Offline-First
description: Bagaimana OZ-POS tetap berjalan tanpa koneksi sama sekali.
category: guides
order: 1
updated: "2026-08-15"
---

## Tidak ada yang berhenti di kasir

Transaksi, shift, pergerakan stok, dan perubahan pengaturan semuanya ditulis
ke database lokal terlebih dahulu. Koneksi yang hilang tidak pernah
memblokir transaksi.

## Antrean offline

Setiap perubahan ditambahkan ke antrean keluar. Saat koneksi kembali, antrean
mengalir sesuai urutan dan server mengonfirmasi setiap item.

## Konflik

Karena setiap register bekerja dengan data lokalnya sendiri dan penggabungan
berbasis urutan, konflik jarang terjadi dan terselesaikan secara deterministik
— perubahan terbaru yang menang untuk setiap catatan.
