---
title: Settings & Data
description: Branding, receipts, currencies, and local data.
category: reference
order: 2
updated: "2026-08-16"
---

## The settings sidebar

Settings is a sidebar of focused screens: **General**, **Appearance**,
**Receipt**, **Cloud Sync**, **Features**, **Data**, **Staff**, **Terminals**,
**Stores**, **Audit Log**, **Offline Queue**, **Shifts**, **Tax Rates**,
**Exchange Rates**, **Promotions**, **Topology**, **Email Reports**, and
**License**. Pin the screens you use often so they stay at the top.

## Store settings

Business name, currency, receipt layout, and hardware defaults are configured
here and synced to every register. **Tax Rates** and **Exchange Rates** add
the rates the checkout and reports use. Receipt settings control paper
width, currency and tax display, rounding, the footer, and the printer —
per workspace, so each screen prints its own way.

## Appearance & devices

**Appearance** sets the theme (dark mode) that the device boots into.
Per-device preferences such as sound volume live on the terminal; see
[Terminals](../terminals/) for what follows the device rather than the user.

## Staff & security

Staff sign in with a PIN or password, and each account has a role — one of
five presets (**owner**, **admin**, **manager**, **staff**, or **auditor**)
— that decides what workspaces and actions are allowed. Sensitive actions
(price overrides, voids, refunds) are PIN-verified, and the **Audit Log**
keeps an immutable record of them. See [User Roles](../user-roles/) for the
full matrix, and [Shifts & Reconciliation](../shifts/) for how the same
trail reconciles cash.

## Data management

The **Data** screen exports, imports, and backs up your data. An export is a
wizard: pick the data types (products, categories, sales, customers, users,
settings) and a date range, and the result is written as an encrypted
`.ozpkg` file. Exports never include passwords, and imports are validated
before anything is replaced. Backups of the local database are the
disaster-recovery copy — see [Offline-First Mode](../offline-mode/) for how
data lives on the device.

## Sync, offline & license

**Cloud Sync** and **Offline Queue** show sync status and what is waiting to
reach the cloud — see [Cloud Sync](../cloud-sync/) and
[Offline-First Mode](../offline-mode/). **License** shows your tier, expiry,
grace period, and limits — see [Licensing & Plans](../licensing/).
