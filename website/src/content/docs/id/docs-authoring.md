---
title: Panduan Gaya Dokumentasi
description: Cara menggunakan callout, tautan, tabel, dan kode saat menulis dokumentasi.
category: reference
order: 3
updated: "2026-08-15"
---

## Callout

Callout menyoroti informasi penting. Tulis blockquote yang baris pertamanya
diawali label tebal — label menentukan warnanya:

> **Catatan:** Informasi umum yang perlu diketahui. Gaya callout bawaan.

> **Info:** Konteks latar belakang atau detail tambahan.

> **Tip:** Pintasan, praktik terbaik, atau pendekatan yang disarankan.

> **Warning:** Sesuatu yang perlu diwaspadai — hasilnya mungkin tidak sesuai harapan.

> **Danger:** Tindakan yang dapat menyebabkan kehilangan data atau merusak instalasi.

Teks apa pun setelah label tebal adalah isi callout:

> **Warning:** Selalu buat cadangan database sebelum menjalankan migrasi.

## Tautan

Tautan relatif antar halaman dokumentasi memakai slug halaman:

```
Lihat panduan [sinkron cloud](../cloud-sync/).
```

Hasilnya: Lihat panduan [sinkron cloud](../cloud-sync/).

## Tabel

Tabel pipa dirender dengan gaya berbingkai:

| Fitur           | Termasuk |
| --------------- | -------- |
| Sinkron cloud   | ✓        |
| Pembayaran QRIS | ✓        |
| Skrip Lua       | ✓        |

## Kode

Kode inline memakai backtick, mis. `Money::from_minor(1000)`. Blok fenced
dirender dalam kotak berbingkai yang bisa digulir:

```rust
let total = cart.total();
let due = total - discount;
```
