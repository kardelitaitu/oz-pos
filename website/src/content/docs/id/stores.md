---
title: Toko & Topologi
description: Modelkan cabang, register, dan gudang dalam satu editor visual.
category: guides
order: 6
updated: "2026-08-16"
---

## Editor topologi

Toko, register, gudang, dan perangkat keras disusun dalam diagram visual —
**Builder Topologi Visual Toko & Workspace**. Node diseret dari palet (atau
ditambah dengan tombol angka) dan dihubungkan dengan kabel di kanvas yang
mendukung zoom, pan, minimap, tata letak otomatis, snap ke grid, serta
undo/redo. Preset siap pakai **Ritel** dan **Resto & KDS** membuat kerangka
toko lengkap dalam satu klik, dan **Uji Simulasi Pesanan** mengirim tiket uji
melalui tata letak sehingga Anda dapat melihat alurnya sebelum diluncurkan.

## Node dan koneksi

Setiap node adalah bagian nyata dari bisnis Anda: **Toko** (profil cabang),
**POS Ritel**, **POS Restoran**, **Layar Dapur (KDS)**, **Gudang**, **Node
Gudang Stok**, dan **Perangkat Keras** (printer dan periferal). Kartu
menampilkan port berjenis — **Lokasi**, **Operasi**, **Stok Masuk/Keluar**,
**Tiket**, dan **Perangkat** — dan saat menghubungkan dua port, editor
menanyakan makna kabel tersebut: pengalihan stok, transfer inventaris,
perutean tiket, koneksi perangkat, atau operasi. Arah kabel berpindah
satu-arah → terbalik → dua-arah, sehingga diagram menunjukkan dengan tepat ke
mana stok, tiket, dan operasi mengalir.

## Validasi

Editor memvalidasi tata letak saat Anda mengerjakan. Panel masalah menandai
kendala secara langsung: tepat satu node cabang per grafik, setiap workspace
terhubung ke cabangnya melalui **Lokasi Masuk**, setiap KDS diumpankan oleh
POS Restoran melalui **Operasi Masuk**, tanpa siklus terarah, dan tanpa node
atau kabel ganda. Peringatan gudang muncul saat penyimpanan penuh atau tidak
ada stok yang dialirkan ke dalamnya.

## Menerapkan perubahan

Menerapkan topologi hanya bisa dilakukan manajer atau pemilik — pengguna lain
melihat kanvas hanya-baca. Terapkan menampilkan ringkasan selisih dari apa
yang akan berubah (dibuat, diperbarui, diarsipkan, berganti tipe, beserta
nomor revisi) sebelum disimpan. Jika topologi berubah di register lain
sementara itu, editor memuat versi terbaru dan meminta Anda menerapkan ulang.

## Cabang, template, dan berbagi

Topologi hidup per cabang. Tampilan **Bandingkan Cabang** menunjukkan apa yang
berbeda antara dua cabang dan dapat memusatkan perhatian pada perbedaannya.
Template menyimpan tata letak untuk dipakai ulang, dan topologi dapat
**diekspor** ke papan klip serta **diimpor** di tempat lain — berguna untuk
menerapkan tata letak yang sama ke setiap cabang.

## Batas paket

Jumlah toko, register, dan gudang ditentukan oleh paket Anda. Editor menandai
apa pun yang melebihi batas sebelum Anda menerapkannya, dan beberapa gudang
atau batas kapasitas gudang memerlukan lisensi Pro Tier.

## Jaga perangkat tetap sinkron

Perangkat menarik topologi saat terhubung kembali, sehingga register baru
muncul di setiap layar tanpa pengaturan manual.
