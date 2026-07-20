# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: inventory-workflows.spec.ts >> Inventory Workflows >> stock counts screen renders container
- Location: e2e\inventory-workflows.spec.ts:28:3

# Error details

```
Error: expect(locator).toBeVisible() failed

Locator: locator('.sc-screen')
Expected: visible
Timeout: 8000ms
Error: element(s) not found

Call log:
  - Expect "toBeVisible" with timeout 8000ms
  - waiting for locator('.sc-screen')

```

```yaml
- link "Lewati ke konten utama":
  - /url: "#app-main-content"
- complementary "nav-main-aria":
  - text: OZ-POS Demo Point of Sale M Manager manager
  - button "Keluar dari Manager"
  - navigation:
    - button "Penjualan"
    - button "Produk"
    - button "Keuangan"
    - button "Pelanggan"
    - button "Laporan"
    - button "Manajemen"
    - button "Pengaturan"
    - button "Pengembang"
  - button "Tutup sidebar"
- main:
  - banner
  - heading "Produk" [level=1]
  - 'button "Select inventory location. Current: Location"': Location
  - button "Open stock alerts"
  - button "Tambah Produk"
  - table "Katalog produk":
    - rowgroup:
      - row "SKU Nama Kategori Harga Barcode Tipe Stok Tindakan":
        - columnheader "SKU"
        - columnheader "Nama"
        - columnheader "Kategori"
        - columnheader "Harga"
        - columnheader "Barcode"
        - columnheader "Tipe"
        - columnheader "Stok"
        - columnheader "Tindakan"
    - rowgroup:
      - row "LATTE Caffè Latte Hot Drinks $ 4,50 4901234567890 restaurant 50 Varian untuk Caffè Latte Edit Caffè Latte Delete Caffè Latte":
        - cell "LATTE"
        - cell "Caffè Latte"
        - cell "Hot Drinks"
        - cell "$ 4,50"
        - cell "4901234567890"
        - cell "restaurant"
        - cell "50"
        - cell "Varian untuk Caffè Latte Edit Caffè Latte Delete Caffè Latte":
          - button "Varian untuk Caffè Latte": Varian
          - button "Edit Caffè Latte": Ubah Caffè Latte
          - button "Delete Caffè Latte": Hapus Caffè Latte
      - row "MLATTE Matcha Latte Hot Drinks $ 5,20 4901234567891 restaurant 30 Varian untuk Matcha Latte Edit Matcha Latte Delete Matcha Latte":
        - cell "MLATTE"
        - cell "Matcha Latte"
        - cell "Hot Drinks"
        - cell "$ 5,20"
        - cell "4901234567891"
        - cell "restaurant"
        - cell "30"
        - cell "Varian untuk Matcha Latte Edit Matcha Latte Delete Matcha Latte":
          - button "Varian untuk Matcha Latte": Varian
          - button "Edit Matcha Latte": Ubah Matcha Latte
          - button "Delete Matcha Latte": Hapus Matcha Latte
      - row "ESPR Espresso Shot Hot Drinks $ 3,00 4901234567892 restaurant 80 Varian untuk Espresso Shot Edit Espresso Shot Delete Espresso Shot":
        - cell "ESPR"
        - cell "Espresso Shot"
        - cell "Hot Drinks"
        - cell "$ 3,00"
        - cell "4901234567892"
        - cell "restaurant"
        - cell "80"
        - cell "Varian untuk Espresso Shot Edit Espresso Shot Delete Espresso Shot":
          - button "Varian untuk Espresso Shot": Varian
          - button "Edit Espresso Shot": Ubah Espresso Shot
          - button "Delete Espresso Shot": Hapus Espresso Shot
      - row "HCHOCO Hot Chocolate Hot Drinks $ 4,20 4901234567893 restaurant 25 Varian untuk Hot Chocolate Edit Hot Chocolate Delete Hot Chocolate":
        - cell "HCHOCO"
        - cell "Hot Chocolate"
        - cell "Hot Drinks"
        - cell "$ 4,20"
        - cell "4901234567893"
        - cell "restaurant"
        - cell "25"
        - cell "Varian untuk Hot Chocolate Edit Hot Chocolate Delete Hot Chocolate":
          - button "Varian untuk Hot Chocolate": Varian
          - button "Edit Hot Chocolate": Ubah Hot Chocolate
          - button "Delete Hot Chocolate": Hapus Hot Chocolate
      - row "ICCOFF Iced Coffee Cold Drinks $ 3,80 4901234567894 restaurant 40 Varian untuk Iced Coffee Edit Iced Coffee Delete Iced Coffee":
        - cell "ICCOFF"
        - cell "Iced Coffee"
        - cell "Cold Drinks"
        - cell "$ 3,80"
        - cell "4901234567894"
        - cell "restaurant"
        - cell "40"
        - cell "Varian untuk Iced Coffee Edit Iced Coffee Delete Iced Coffee":
          - button "Varian untuk Iced Coffee": Varian
          - button "Edit Iced Coffee": Ubah Iced Coffee
          - button "Delete Iced Coffee": Hapus Iced Coffee
      - row "ICTEA Iced Tea Cold Drinks $ 2,50 4901234567895 restaurant 60 Varian untuk Iced Tea Edit Iced Tea Delete Iced Tea":
        - cell "ICTEA"
        - cell "Iced Tea"
        - cell "Cold Drinks"
        - cell "$ 2,50"
        - cell "4901234567895"
        - cell "restaurant"
        - cell "60"
        - cell "Varian untuk Iced Tea Edit Iced Tea Delete Iced Tea":
          - button "Varian untuk Iced Tea": Varian
          - button "Edit Iced Tea": Ubah Iced Tea
          - button "Delete Iced Tea": Hapus Iced Tea
      - row "JUICE-O Orange Juice Cold Drinks $ 3,50 4901234567904 restaurant 20 Varian untuk Orange Juice Edit Orange Juice Delete Orange Juice":
        - cell "JUICE-O"
        - cell "Orange Juice"
        - cell "Cold Drinks"
        - cell "$ 3,50"
        - cell "4901234567904"
        - cell "restaurant"
        - cell "20"
        - cell "Varian untuk Orange Juice Edit Orange Juice Delete Orange Juice":
          - button "Varian untuk Orange Juice": Varian
          - button "Edit Orange Juice": Ubah Orange Juice
          - button "Delete Orange Juice": Hapus Orange Juice
      - row "LEMONADE Lemonade Cold Drinks $ 3,00 4901234567897 restaurant 35 Varian untuk Lemonade Edit Lemonade Delete Lemonade":
        - cell "LEMONADE"
        - cell "Lemonade"
        - cell "Cold Drinks"
        - cell "$ 3,00"
        - cell "4901234567897"
        - cell "restaurant"
        - cell "35"
        - cell "Varian untuk Lemonade Edit Lemonade Delete Lemonade":
          - button "Varian untuk Lemonade": Varian
          - button "Edit Lemonade": Ubah Lemonade
          - button "Delete Lemonade": Hapus Lemonade
      - row "PBAGEL Plain Bagel Food $ 2,50 4901234567898 restaurant 15 Varian untuk Plain Bagel Edit Plain Bagel Delete Plain Bagel":
        - cell "PBAGEL"
        - cell "Plain Bagel"
        - cell "Food"
        - cell "$ 2,50"
        - cell "4901234567898"
        - cell "restaurant"
        - cell "15"
        - cell "Varian untuk Plain Bagel Edit Plain Bagel Delete Plain Bagel":
          - button "Varian untuk Plain Bagel": Varian
          - button "Edit Plain Bagel": Ubah Plain Bagel
          - button "Delete Plain Bagel": Hapus Plain Bagel
      - row "SBAGEL Sesame Bagel Food $ 2,80 4901234567899 restaurant 12 Varian untuk Sesame Bagel Edit Sesame Bagel Delete Sesame Bagel":
        - cell "SBAGEL"
        - cell "Sesame Bagel"
        - cell "Food"
        - cell "$ 2,80"
        - cell "4901234567899"
        - cell "restaurant"
        - cell "12"
        - cell "Varian untuk Sesame Bagel Edit Sesame Bagel Delete Sesame Bagel":
          - button "Varian untuk Sesame Bagel": Varian
          - button "Edit Sesame Bagel": Ubah Sesame Bagel
          - button "Delete Sesame Bagel": Hapus Sesame Bagel
      - row "CROISS Butter Croissant Food $ 3,20 4901234567800 restaurant 18 Varian untuk Butter Croissant Edit Butter Croissant Delete Butter Croissant":
        - cell "CROISS"
        - cell "Butter Croissant"
        - cell "Food"
        - cell "$ 3,20"
        - cell "4901234567800"
        - cell "restaurant"
        - cell "18"
        - cell "Varian untuk Butter Croissant Edit Butter Croissant Delete Butter Croissant":
          - button "Varian untuk Butter Croissant": Varian
          - button "Edit Butter Croissant": Ubah Butter Croissant
          - button "Delete Butter Croissant": Hapus Butter Croissant
      - row "CSAND Chicken Sandwich Food $ 5,50 4901234567801 restaurant 10 Varian untuk Chicken Sandwich Edit Chicken Sandwich Delete Chicken Sandwich":
        - cell "CSAND"
        - cell "Chicken Sandwich"
        - cell "Food"
        - cell "$ 5,50"
        - cell "4901234567801"
        - cell "restaurant"
        - cell "10"
        - cell "Varian untuk Chicken Sandwich Edit Chicken Sandwich Delete Chicken Sandwich":
          - button "Varian untuk Chicken Sandwich": Varian
          - button "Edit Chicken Sandwich": Ubah Chicken Sandwich
          - button "Delete Chicken Sandwich": Hapus Chicken Sandwich
      - row "VSAND Veggie Sandwich Food $ 4,80 4901234567802 restaurant 8 Varian untuk Veggie Sandwich Edit Veggie Sandwich Delete Veggie Sandwich":
        - cell "VSAND"
        - cell "Veggie Sandwich"
        - cell "Food"
        - cell "$ 4,80"
        - cell "4901234567802"
        - cell "restaurant"
        - cell "8"
        - cell "Varian untuk Veggie Sandwich Edit Veggie Sandwich Delete Veggie Sandwich":
          - button "Varian untuk Veggie Sandwich": Varian
          - button "Edit Veggie Sandwich": Ubah Veggie Sandwich
          - button "Delete Veggie Sandwich": Hapus Veggie Sandwich
      - row "WATER-S Sparkling Water Cold Drinks $ 1,80 4901234567803 restaurant 150 Varian untuk Sparkling Water Edit Sparkling Water Delete Sparkling Water":
        - cell "WATER-S"
        - cell "Sparkling Water"
        - cell "Cold Drinks"
        - cell "$ 1,80"
        - cell "4901234567803"
        - cell "restaurant"
        - cell "150"
        - cell "Varian untuk Sparkling Water Edit Sparkling Water Delete Sparkling Water":
          - button "Varian untuk Sparkling Water": Varian
          - button "Edit Sparkling Water": Ubah Sparkling Water
          - button "Delete Sparkling Water": Hapus Sparkling Water
      - row "BROWNIE Fudge Brownie Snacks $ 3,00 4901234567804 restaurant 0 Varian untuk Fudge Brownie Edit Fudge Brownie Delete Fudge Brownie":
        - cell "BROWNIE"
        - cell "Fudge Brownie"
        - cell "Snacks"
        - cell "$ 3,00"
        - cell "4901234567804"
        - cell "restaurant"
        - cell "0"
        - cell "Varian untuk Fudge Brownie Edit Fudge Brownie Delete Fudge Brownie":
          - button "Varian untuk Fudge Brownie": Varian
          - button "Edit Fudge Brownie": Ubah Fudge Brownie
          - button "Delete Fudge Brownie": Hapus Fudge Brownie
      - row "CMUFFIN Chocolate Muffin Snacks $ 2,80 4901234567805 restaurant 0 Varian untuk Chocolate Muffin Edit Chocolate Muffin Delete Chocolate Muffin":
        - cell "CMUFFIN"
        - cell "Chocolate Muffin"
        - cell "Snacks"
        - cell "$ 2,80"
        - cell "4901234567805"
        - cell "restaurant"
        - cell "0"
        - cell "Varian untuk Chocolate Muffin Edit Chocolate Muffin Delete Chocolate Muffin":
          - button "Varian untuk Chocolate Muffin": Varian
          - button "Edit Chocolate Muffin": Ubah Chocolate Muffin
          - button "Delete Chocolate Muffin": Hapus Chocolate Muffin
      - row "NUTS Mixed Nuts Snacks $ 4,00 4901234567806 restaurant 22 Varian untuk Mixed Nuts Edit Mixed Nuts Delete Mixed Nuts":
        - cell "NUTS"
        - cell "Mixed Nuts"
        - cell "Snacks"
        - cell "$ 4,00"
        - cell "4901234567806"
        - cell "restaurant"
        - cell "22"
        - cell "Varian untuk Mixed Nuts Edit Mixed Nuts Delete Mixed Nuts":
          - button "Varian untuk Mixed Nuts": Varian
          - button "Edit Mixed Nuts": Ubah Mixed Nuts
          - button "Delete Mixed Nuts": Hapus Mixed Nuts
      - row "CHIPS Potato Chips Snacks $ 2,00 4901234567807 restaurant 55 Varian untuk Potato Chips Edit Potato Chips Delete Potato Chips":
        - cell "CHIPS"
        - cell "Potato Chips"
        - cell "Snacks"
        - cell "$ 2,00"
        - cell "4901234567807"
        - cell "restaurant"
        - cell "55"
        - cell "Varian untuk Potato Chips Edit Potato Chips Delete Potato Chips":
          - button "Varian untuk Potato Chips": Varian
          - button "Edit Potato Chips": Ubah Potato Chips
          - button "Delete Potato Chips": Hapus Potato Chips
- status "Application status":
  - text: OZ-POS Enterprise v0.0.14 Proprietary License
  - button "Ganti Pengguna"
  - button "Ganti Ruang Kerja"
  - button "Beralih ke mode terang": Alihkan tema
- toolbar "Developer tools":
  - text: DevTools
  - paragraph: Theme
  - radiogroup "Theme selector":
    - radio "Glass theme" [checked]: Glass
    - radio "Light theme": Light
    - radio "Dark theme": Dark
  - text: Glass
```

# Test source

```ts
  1  | import { test, expect } from '@playwright/test';
  2  | import { loginAs, selectWorkspace, WORKSPACES, navigateTo } from './helpers';
  3  | 
  4  | /**
  5  |  * E2E: Inventory Workflows — Stock Counts, Transfers, Purchase Orders
  6  |  *
  7  |  * Covers inventory operations with zero prior E2E coverage.
  8  |  * All tests use hard assertions — no soft guards.
  9  |  *
  10 |  * Routes (all accessible from Inventory workspace):
  11 |  *   #/stock-counts      → StockCountsScreen (.sc-screen)
  12 |  *   #/stock-transfers   → StockTransfersScreen (.stock-transfers)
  13 |  *   #/purchase-orders   → PurchaseOrdersScreen (.po-screen)
  14 |  *   #/suppliers         → SuppliersScreen (.suppliers-screen)
  15 |  *
  16 |  * Mock data: all lists return empty arrays — screens render
  17 |  * with empty-state placeholders.
  18 |  */
  19 | 
  20 | test.describe('Inventory Workflows', () => {
  21 |   test.beforeEach(async ({ page }) => {
  22 |     await loginAs(page, 'admin', '9999');
  23 |     await selectWorkspace(page, WORKSPACES.INVENTORY);
  24 |   });
  25 | 
  26 |   // ── Stock Counts ────────────────────────────────────────
  27 | 
  28 |   test('stock counts screen renders container', async ({ page }) => {
  29 |     await navigateTo(page, 'stock-counts');
  30 | 
> 31 |     await expect(page.locator('.sc-screen')).toBeVisible({ timeout: 8_000 });
     |                                              ^ Error: expect(locator).toBeVisible() failed
  32 |   });
  33 | 
  34 |   // ── Stock Transfers ─────────────────────────────────────
  35 | 
  36 |   test('stock transfers screen renders with title', async ({ page }) => {
  37 |     await navigateTo(page, 'stock-transfers');
  38 | 
  39 |     await expect(page.locator('.stock-transfers')).toBeVisible({ timeout: 8_000 });
  40 |     await expect(page.locator('.stock-transfers-title')).toContainText('Stock Transfer');
  41 |   });
  42 | 
  43 |   // ── Purchase Orders ─────────────────────────────────────
  44 | 
  45 |   test('purchase orders screen renders container', async ({ page }) => {
  46 |     await navigateTo(page, 'purchase-orders');
  47 | 
  48 |     await expect(page.locator('.po-screen')).toBeVisible({ timeout: 8_000 });
  49 |   });
  50 | 
  51 |   // ── Suppliers ───────────────────────────────────────────
  52 | 
  53 |   test('suppliers screen renders with table', async ({ page }) => {
  54 |     await navigateTo(page, 'suppliers');
  55 | 
  56 |     await expect(page.locator('.suppliers-screen')).toBeVisible({ timeout: 8_000 });
  57 |     await expect(page.locator('.suppliers-title')).toContainText('Supplier');
  58 | 
  59 |     // Table must render (even if empty).
  60 |     await expect(page.locator('.suppliers-table')).toBeVisible({ timeout: 5_000 });
  61 |   });
  62 | });
  63 | 
```