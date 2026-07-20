# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: product.spec.ts >> Product Management >> edit product opens modal with pre-filled fields
- Location: e2e\product.spec.ts:92:3

# Error details

```
Test timeout of 30000ms exceeded.
```

```
Error: locator.click: Test timeout of 30000ms exceeded.
Call log:
  - waiting for locator('.product-mgmt-action-btn').filter({ hasText: 'Edit' }).first()

```

# Page snapshot

```yaml
- generic [ref=e2]:
  - generic [ref=e3]:
    - link "Lewati ke konten utama" [ref=e4] [cursor=pointer]:
      - /url: "#app-main-content"
    - generic [ref=e5]:
      - complementary "nav-main-aria" [ref=e6]:
        - generic [ref=e7]:
          - generic [ref=e9]:
            - img [ref=e11]
            - generic [ref=e13]:
              - generic [ref=e14]: OZ-POS Demo
              - generic [ref=e15]: Point of Sale
          - generic "Masuk sebagai Manager, manager" [ref=e16]:
            - generic [ref=e17]: M
            - generic:
              - generic: Manager
              - generic [ref=e18]: manager
            - generic [ref=e19]:
              - button "Keluar dari Manager":
                - img
        - navigation [ref=e20]:
          - button "Penjualan" [ref=e22] [cursor=pointer]:
            - generic [ref=e23]: Penjualan
            - img [ref=e24]
          - button "Produk" [ref=e27] [cursor=pointer]:
            - generic [ref=e28]: Produk
            - img [ref=e29]
          - button "Keuangan" [ref=e32] [cursor=pointer]:
            - generic [ref=e33]: Keuangan
            - img [ref=e34]
          - button "Pelanggan" [ref=e37] [cursor=pointer]:
            - generic [ref=e38]: Pelanggan
            - img [ref=e39]
          - button "Laporan" [ref=e42] [cursor=pointer]:
            - generic [ref=e43]: Laporan
            - img [ref=e44]
          - button "Manajemen" [ref=e47] [cursor=pointer]:
            - generic [ref=e48]: Manajemen
            - img [ref=e49]
          - button "Pengaturan" [ref=e52] [cursor=pointer]:
            - generic [ref=e53]: Pengaturan
            - img [ref=e54]
          - button "Pengembang" [ref=e57] [cursor=pointer]:
            - generic [ref=e58]: Pengembang
            - img [ref=e59]
        - button "Tutup sidebar" [ref=e62] [cursor=pointer]:
          - img [ref=e63]
      - main [ref=e65]:
        - banner [ref=e66]:
          - tooltip [ref=e67]: Tutup sidebar
        - generic [ref=e69]:
          - generic [ref=e70]:
            - heading "Produk" [level=1] [ref=e71]
            - generic [ref=e72]:
              - 'button "Select inventory location. Current: Location" [ref=e74] [cursor=pointer]':
                - img [ref=e75]
                - generic [ref=e78]: Location
                - img [ref=e79]
              - button "Open stock alerts" [ref=e81] [cursor=pointer]:
                - img [ref=e82]
              - button "Tambah Produk" [ref=e85] [cursor=pointer]
          - table "Katalog produk" [ref=e87]:
            - rowgroup [ref=e88]:
              - row "SKU Nama Kategori Harga Barcode Tipe Stok Tindakan" [ref=e89]:
                - columnheader "SKU" [ref=e90]
                - columnheader "Nama" [ref=e91]
                - columnheader "Kategori" [ref=e92]
                - columnheader "Harga" [ref=e93]
                - columnheader "Barcode" [ref=e94]
                - columnheader "Tipe" [ref=e95]
                - columnheader "Stok" [ref=e96]
                - columnheader "Tindakan" [ref=e97]
            - rowgroup [ref=e98]:
              - row "LATTE Caffè Latte Hot Drinks $ 4,50 4901234567890 restaurant 50 Varian untuk Caffè Latte Edit Caffè Latte Delete Caffè Latte" [ref=e99]:
                - cell "LATTE" [ref=e100]
                - cell "Caffè Latte" [ref=e101]
                - cell "Hot Drinks" [ref=e102]
                - cell "$ 4,50" [ref=e103]
                - cell "4901234567890" [ref=e104]
                - cell "restaurant" [ref=e105]:
                  - generic [ref=e106]: restaurant
                - cell "50" [ref=e107]
                - cell "Varian untuk Caffè Latte Edit Caffè Latte Delete Caffè Latte" [ref=e108]:
                  - button "Varian untuk Caffè Latte" [ref=e109] [cursor=pointer]:
                    - generic [ref=e110]: Varian
                  - button "Edit Caffè Latte" [ref=e111] [cursor=pointer]: Ubah Caffè Latte
                  - button "Delete Caffè Latte" [ref=e112] [cursor=pointer]: Hapus Caffè Latte
              - row "MLATTE Matcha Latte Hot Drinks $ 5,20 4901234567891 restaurant 30 Varian untuk Matcha Latte Edit Matcha Latte Delete Matcha Latte" [ref=e113]:
                - cell "MLATTE" [ref=e114]
                - cell "Matcha Latte" [ref=e115]
                - cell "Hot Drinks" [ref=e116]
                - cell "$ 5,20" [ref=e117]
                - cell "4901234567891" [ref=e118]
                - cell "restaurant" [ref=e119]:
                  - generic [ref=e120]: restaurant
                - cell "30" [ref=e121]
                - cell "Varian untuk Matcha Latte Edit Matcha Latte Delete Matcha Latte" [ref=e122]:
                  - button "Varian untuk Matcha Latte" [ref=e123] [cursor=pointer]:
                    - generic [ref=e124]: Varian
                  - button "Edit Matcha Latte" [ref=e125] [cursor=pointer]: Ubah Matcha Latte
                  - button "Delete Matcha Latte" [ref=e126] [cursor=pointer]: Hapus Matcha Latte
              - row "ESPR Espresso Shot Hot Drinks $ 3,00 4901234567892 restaurant 80 Varian untuk Espresso Shot Edit Espresso Shot Delete Espresso Shot" [ref=e127]:
                - cell "ESPR" [ref=e128]
                - cell "Espresso Shot" [ref=e129]
                - cell "Hot Drinks" [ref=e130]
                - cell "$ 3,00" [ref=e131]
                - cell "4901234567892" [ref=e132]
                - cell "restaurant" [ref=e133]:
                  - generic [ref=e134]: restaurant
                - cell "80" [ref=e135]
                - cell "Varian untuk Espresso Shot Edit Espresso Shot Delete Espresso Shot" [ref=e136]:
                  - button "Varian untuk Espresso Shot" [ref=e137] [cursor=pointer]:
                    - generic [ref=e138]: Varian
                  - button "Edit Espresso Shot" [ref=e139] [cursor=pointer]: Ubah Espresso Shot
                  - button "Delete Espresso Shot" [ref=e140] [cursor=pointer]: Hapus Espresso Shot
              - row "HCHOCO Hot Chocolate Hot Drinks $ 4,20 4901234567893 restaurant 25 Varian untuk Hot Chocolate Edit Hot Chocolate Delete Hot Chocolate" [ref=e141]:
                - cell "HCHOCO" [ref=e142]
                - cell "Hot Chocolate" [ref=e143]
                - cell "Hot Drinks" [ref=e144]
                - cell "$ 4,20" [ref=e145]
                - cell "4901234567893" [ref=e146]
                - cell "restaurant" [ref=e147]:
                  - generic [ref=e148]: restaurant
                - cell "25" [ref=e149]
                - cell "Varian untuk Hot Chocolate Edit Hot Chocolate Delete Hot Chocolate" [ref=e150]:
                  - button "Varian untuk Hot Chocolate" [ref=e151] [cursor=pointer]:
                    - generic [ref=e152]: Varian
                  - button "Edit Hot Chocolate" [ref=e153] [cursor=pointer]: Ubah Hot Chocolate
                  - button "Delete Hot Chocolate" [ref=e154] [cursor=pointer]: Hapus Hot Chocolate
              - row "ICCOFF Iced Coffee Cold Drinks $ 3,80 4901234567894 restaurant 40 Varian untuk Iced Coffee Edit Iced Coffee Delete Iced Coffee" [ref=e155]:
                - cell "ICCOFF" [ref=e156]
                - cell "Iced Coffee" [ref=e157]
                - cell "Cold Drinks" [ref=e158]
                - cell "$ 3,80" [ref=e159]
                - cell "4901234567894" [ref=e160]
                - cell "restaurant" [ref=e161]:
                  - generic [ref=e162]: restaurant
                - cell "40" [ref=e163]
                - cell "Varian untuk Iced Coffee Edit Iced Coffee Delete Iced Coffee" [ref=e164]:
                  - button "Varian untuk Iced Coffee" [ref=e165] [cursor=pointer]:
                    - generic [ref=e166]: Varian
                  - button "Edit Iced Coffee" [ref=e167] [cursor=pointer]: Ubah Iced Coffee
                  - button "Delete Iced Coffee" [ref=e168] [cursor=pointer]: Hapus Iced Coffee
              - row "ICTEA Iced Tea Cold Drinks $ 2,50 4901234567895 restaurant 60 Varian untuk Iced Tea Edit Iced Tea Delete Iced Tea" [ref=e169]:
                - cell "ICTEA" [ref=e170]
                - cell "Iced Tea" [ref=e171]
                - cell "Cold Drinks" [ref=e172]
                - cell "$ 2,50" [ref=e173]
                - cell "4901234567895" [ref=e174]
                - cell "restaurant" [ref=e175]:
                  - generic [ref=e176]: restaurant
                - cell "60" [ref=e177]
                - cell "Varian untuk Iced Tea Edit Iced Tea Delete Iced Tea" [ref=e178]:
                  - button "Varian untuk Iced Tea" [ref=e179] [cursor=pointer]:
                    - generic [ref=e180]: Varian
                  - button "Edit Iced Tea" [ref=e181] [cursor=pointer]: Ubah Iced Tea
                  - button "Delete Iced Tea" [ref=e182] [cursor=pointer]: Hapus Iced Tea
              - row "JUICE-O Orange Juice Cold Drinks $ 3,50 4901234567904 restaurant 20 Varian untuk Orange Juice Edit Orange Juice Delete Orange Juice" [ref=e183]:
                - cell "JUICE-O" [ref=e184]
                - cell "Orange Juice" [ref=e185]
                - cell "Cold Drinks" [ref=e186]
                - cell "$ 3,50" [ref=e187]
                - cell "4901234567904" [ref=e188]
                - cell "restaurant" [ref=e189]:
                  - generic [ref=e190]: restaurant
                - cell "20" [ref=e191]
                - cell "Varian untuk Orange Juice Edit Orange Juice Delete Orange Juice" [ref=e192]:
                  - button "Varian untuk Orange Juice" [ref=e193] [cursor=pointer]:
                    - generic [ref=e194]: Varian
                  - button "Edit Orange Juice" [ref=e195] [cursor=pointer]: Ubah Orange Juice
                  - button "Delete Orange Juice" [ref=e196] [cursor=pointer]: Hapus Orange Juice
              - row "LEMONADE Lemonade Cold Drinks $ 3,00 4901234567897 restaurant 35 Varian untuk Lemonade Edit Lemonade Delete Lemonade" [ref=e197]:
                - cell "LEMONADE" [ref=e198]
                - cell "Lemonade" [ref=e199]
                - cell "Cold Drinks" [ref=e200]
                - cell "$ 3,00" [ref=e201]
                - cell "4901234567897" [ref=e202]
                - cell "restaurant" [ref=e203]:
                  - generic [ref=e204]: restaurant
                - cell "35" [ref=e205]
                - cell "Varian untuk Lemonade Edit Lemonade Delete Lemonade" [ref=e206]:
                  - button "Varian untuk Lemonade" [ref=e207] [cursor=pointer]:
                    - generic [ref=e208]: Varian
                  - button "Edit Lemonade" [ref=e209] [cursor=pointer]: Ubah Lemonade
                  - button "Delete Lemonade" [ref=e210] [cursor=pointer]: Hapus Lemonade
              - row "PBAGEL Plain Bagel Food $ 2,50 4901234567898 restaurant 15 Varian untuk Plain Bagel Edit Plain Bagel Delete Plain Bagel" [ref=e211]:
                - cell "PBAGEL" [ref=e212]
                - cell "Plain Bagel" [ref=e213]
                - cell "Food" [ref=e214]
                - cell "$ 2,50" [ref=e215]
                - cell "4901234567898" [ref=e216]
                - cell "restaurant" [ref=e217]:
                  - generic [ref=e218]: restaurant
                - cell "15" [ref=e219]
                - cell "Varian untuk Plain Bagel Edit Plain Bagel Delete Plain Bagel" [ref=e220]:
                  - button "Varian untuk Plain Bagel" [ref=e221] [cursor=pointer]:
                    - generic [ref=e222]: Varian
                  - button "Edit Plain Bagel" [ref=e223] [cursor=pointer]: Ubah Plain Bagel
                  - button "Delete Plain Bagel" [ref=e224] [cursor=pointer]: Hapus Plain Bagel
              - row "SBAGEL Sesame Bagel Food $ 2,80 4901234567899 restaurant 12 Varian untuk Sesame Bagel Edit Sesame Bagel Delete Sesame Bagel" [ref=e225]:
                - cell "SBAGEL" [ref=e226]
                - cell "Sesame Bagel" [ref=e227]
                - cell "Food" [ref=e228]
                - cell "$ 2,80" [ref=e229]
                - cell "4901234567899" [ref=e230]
                - cell "restaurant" [ref=e231]:
                  - generic [ref=e232]: restaurant
                - cell "12" [ref=e233]
                - cell "Varian untuk Sesame Bagel Edit Sesame Bagel Delete Sesame Bagel" [ref=e234]:
                  - button "Varian untuk Sesame Bagel" [ref=e235] [cursor=pointer]:
                    - generic [ref=e236]: Varian
                  - button "Edit Sesame Bagel" [ref=e237] [cursor=pointer]: Ubah Sesame Bagel
                  - button "Delete Sesame Bagel" [ref=e238] [cursor=pointer]: Hapus Sesame Bagel
              - row "CROISS Butter Croissant Food $ 3,20 4901234567800 restaurant 18 Varian untuk Butter Croissant Edit Butter Croissant Delete Butter Croissant" [ref=e239]:
                - cell "CROISS" [ref=e240]
                - cell "Butter Croissant" [ref=e241]
                - cell "Food" [ref=e242]
                - cell "$ 3,20" [ref=e243]
                - cell "4901234567800" [ref=e244]
                - cell "restaurant" [ref=e245]:
                  - generic [ref=e246]: restaurant
                - cell "18" [ref=e247]
                - cell "Varian untuk Butter Croissant Edit Butter Croissant Delete Butter Croissant" [ref=e248]:
                  - button "Varian untuk Butter Croissant" [ref=e249] [cursor=pointer]:
                    - generic [ref=e250]: Varian
                  - button "Edit Butter Croissant" [ref=e251] [cursor=pointer]: Ubah Butter Croissant
                  - button "Delete Butter Croissant" [ref=e252] [cursor=pointer]: Hapus Butter Croissant
              - row "CSAND Chicken Sandwich Food $ 5,50 4901234567801 restaurant 10 Varian untuk Chicken Sandwich Edit Chicken Sandwich Delete Chicken Sandwich" [ref=e253]:
                - cell "CSAND" [ref=e254]
                - cell "Chicken Sandwich" [ref=e255]
                - cell "Food" [ref=e256]
                - cell "$ 5,50" [ref=e257]
                - cell "4901234567801" [ref=e258]
                - cell "restaurant" [ref=e259]:
                  - generic [ref=e260]: restaurant
                - cell "10" [ref=e261]
                - cell "Varian untuk Chicken Sandwich Edit Chicken Sandwich Delete Chicken Sandwich" [ref=e262]:
                  - button "Varian untuk Chicken Sandwich" [ref=e263] [cursor=pointer]:
                    - generic [ref=e264]: Varian
                  - button "Edit Chicken Sandwich" [ref=e265] [cursor=pointer]: Ubah Chicken Sandwich
                  - button "Delete Chicken Sandwich" [ref=e266] [cursor=pointer]: Hapus Chicken Sandwich
              - row "VSAND Veggie Sandwich Food $ 4,80 4901234567802 restaurant 8 Varian untuk Veggie Sandwich Edit Veggie Sandwich Delete Veggie Sandwich" [ref=e267]:
                - cell "VSAND" [ref=e268]
                - cell "Veggie Sandwich" [ref=e269]
                - cell "Food" [ref=e270]
                - cell "$ 4,80" [ref=e271]
                - cell "4901234567802" [ref=e272]
                - cell "restaurant" [ref=e273]:
                  - generic [ref=e274]: restaurant
                - cell "8" [ref=e275]
                - cell "Varian untuk Veggie Sandwich Edit Veggie Sandwich Delete Veggie Sandwich" [ref=e276]:
                  - button "Varian untuk Veggie Sandwich" [ref=e277] [cursor=pointer]:
                    - generic [ref=e278]: Varian
                  - button "Edit Veggie Sandwich" [ref=e279] [cursor=pointer]: Ubah Veggie Sandwich
                  - button "Delete Veggie Sandwich" [ref=e280] [cursor=pointer]: Hapus Veggie Sandwich
              - row "WATER-S Sparkling Water Cold Drinks $ 1,80 4901234567803 restaurant 150 Varian untuk Sparkling Water Edit Sparkling Water Delete Sparkling Water" [ref=e281]:
                - cell "WATER-S" [ref=e282]
                - cell "Sparkling Water" [ref=e283]
                - cell "Cold Drinks" [ref=e284]
                - cell "$ 1,80" [ref=e285]
                - cell "4901234567803" [ref=e286]
                - cell "restaurant" [ref=e287]:
                  - generic [ref=e288]: restaurant
                - cell "150" [ref=e289]
                - cell "Varian untuk Sparkling Water Edit Sparkling Water Delete Sparkling Water" [ref=e290]:
                  - button "Varian untuk Sparkling Water" [ref=e291] [cursor=pointer]:
                    - generic [ref=e292]: Varian
                  - button "Edit Sparkling Water" [ref=e293] [cursor=pointer]: Ubah Sparkling Water
                  - button "Delete Sparkling Water" [ref=e294] [cursor=pointer]: Hapus Sparkling Water
              - row "BROWNIE Fudge Brownie Snacks $ 3,00 4901234567804 restaurant 0 Varian untuk Fudge Brownie Edit Fudge Brownie Delete Fudge Brownie" [ref=e295]:
                - cell "BROWNIE" [ref=e296]
                - cell "Fudge Brownie" [ref=e297]
                - cell "Snacks" [ref=e298]
                - cell "$ 3,00" [ref=e299]
                - cell "4901234567804" [ref=e300]
                - cell "restaurant" [ref=e301]:
                  - generic [ref=e302]: restaurant
                - cell "0" [ref=e303]
                - cell "Varian untuk Fudge Brownie Edit Fudge Brownie Delete Fudge Brownie" [ref=e304]:
                  - button "Varian untuk Fudge Brownie" [ref=e305] [cursor=pointer]:
                    - generic [ref=e306]: Varian
                  - button "Edit Fudge Brownie" [ref=e307] [cursor=pointer]: Ubah Fudge Brownie
                  - button "Delete Fudge Brownie" [ref=e308] [cursor=pointer]: Hapus Fudge Brownie
              - row "CMUFFIN Chocolate Muffin Snacks $ 2,80 4901234567805 restaurant 0 Varian untuk Chocolate Muffin Edit Chocolate Muffin Delete Chocolate Muffin" [ref=e309]:
                - cell "CMUFFIN" [ref=e310]
                - cell "Chocolate Muffin" [ref=e311]
                - cell "Snacks" [ref=e312]
                - cell "$ 2,80" [ref=e313]
                - cell "4901234567805" [ref=e314]
                - cell "restaurant" [ref=e315]:
                  - generic [ref=e316]: restaurant
                - cell "0" [ref=e317]
                - cell "Varian untuk Chocolate Muffin Edit Chocolate Muffin Delete Chocolate Muffin" [ref=e318]:
                  - button "Varian untuk Chocolate Muffin" [ref=e319] [cursor=pointer]:
                    - generic [ref=e320]: Varian
                  - button "Edit Chocolate Muffin" [ref=e321] [cursor=pointer]: Ubah Chocolate Muffin
                  - button "Delete Chocolate Muffin" [ref=e322] [cursor=pointer]: Hapus Chocolate Muffin
              - row "NUTS Mixed Nuts Snacks $ 4,00 4901234567806 restaurant 22 Varian untuk Mixed Nuts Edit Mixed Nuts Delete Mixed Nuts" [ref=e323]:
                - cell "NUTS" [ref=e324]
                - cell "Mixed Nuts" [ref=e325]
                - cell "Snacks" [ref=e326]
                - cell "$ 4,00" [ref=e327]
                - cell "4901234567806" [ref=e328]
                - cell "restaurant" [ref=e329]:
                  - generic [ref=e330]: restaurant
                - cell "22" [ref=e331]
                - cell "Varian untuk Mixed Nuts Edit Mixed Nuts Delete Mixed Nuts" [ref=e332]:
                  - button "Varian untuk Mixed Nuts" [ref=e333] [cursor=pointer]:
                    - generic [ref=e334]: Varian
                  - button "Edit Mixed Nuts" [ref=e335] [cursor=pointer]: Ubah Mixed Nuts
                  - button "Delete Mixed Nuts" [ref=e336] [cursor=pointer]: Hapus Mixed Nuts
              - row "CHIPS Potato Chips Snacks $ 2,00 4901234567807 restaurant 55 Varian untuk Potato Chips Edit Potato Chips Delete Potato Chips" [ref=e337]:
                - cell "CHIPS" [ref=e338]
                - cell "Potato Chips" [ref=e339]
                - cell "Snacks" [ref=e340]
                - cell "$ 2,00" [ref=e341]
                - cell "4901234567807" [ref=e342]
                - cell "restaurant" [ref=e343]:
                  - generic [ref=e344]: restaurant
                - cell "55" [ref=e345]
                - cell "Varian untuk Potato Chips Edit Potato Chips Delete Potato Chips" [ref=e346]:
                  - button "Varian untuk Potato Chips" [ref=e347] [cursor=pointer]:
                    - generic [ref=e348]: Varian
                  - button "Edit Potato Chips" [ref=e349] [cursor=pointer]: Ubah Potato Chips
                  - button "Delete Potato Chips" [ref=e350] [cursor=pointer]: Hapus Potato Chips
    - status "Application status" [ref=e351]:
      - generic [ref=e352]:
        - generic [ref=e353]:
          - generic [ref=e356]: OZ-POS Enterprise v0.0.14
          - tooltip [ref=e357]: Backend terputus
        - generic [ref=e359]: Proprietary License
      - generic [ref=e360]:
        - generic [ref=e361]:
          - button "Ganti Pengguna" [ref=e362] [cursor=pointer]:
            - generic [ref=e367]: Ganti Pengguna
          - tooltip [ref=e368]: Ganti Pengguna
        - generic [ref=e370]:
          - button "Ganti Ruang Kerja" [ref=e371] [cursor=pointer]:
            - generic [ref=e373]: Ganti Ruang Kerja
          - tooltip [ref=e374]: Ganti Ruang Kerja
        - generic [ref=e376]:
          - button "Beralih ke mode terang" [ref=e377] [cursor=pointer]:
            - generic [ref=e378]: Alihkan tema
            - img [ref=e379]
          - tooltip [ref=e385]: Alihkan tema
  - toolbar "Developer tools" [ref=e386]:
    - generic [ref=e388]: DevTools
    - generic [ref=e389]:
      - paragraph [ref=e390]: Theme
      - radiogroup "Theme selector" [ref=e391]:
        - radio "Glass theme" [checked] [ref=e392] [cursor=pointer]:
          - img [ref=e393]
          - generic [ref=e397]: Glass
        - radio "Light theme" [ref=e398] [cursor=pointer]:
          - img [ref=e399]
          - generic [ref=e405]: Light
        - radio "Dark theme" [ref=e406] [cursor=pointer]:
          - img [ref=e407]
          - generic [ref=e409]: Dark
      - generic [ref=e411]: Glass
```

# Test source

```ts
  1   | import { test, expect } from '@playwright/test';
  2   | import { loginAs, selectWorkspace, WORKSPACES } from './helpers';
  3   | 
  4   | /**
  5   |  * E2E: Product Management — Hard Assertions (E2E-16 through E2E-19)
  6   |  *
  7   |  * Tests the inventory product management screen with deterministic
  8   |  * assertions. All `if` guards removed — tests hard-fail on regressions.
  9   |  *
  10  |  * Dev-mock returns 18 products (MOCK_PRODUCTS in tauri-api.ts).
  11  |  *
  12  |  * CSS contract (ProductManagementScreen.tsx):
  13  |  *   .product-mgmt               — container
  14  |  *   .product-mgmt-table         — product table
  15  |  *   .product-mgmt-cell-sku      — SKU cell
  16  |  *   .product-mgmt-cell-price    — price cell
  17  |  *   .product-mgmt-header-actions — header with Add Product button
  18  |  *   .product-mgmt-overlay       — create/edit modal backdrop
  19  |  *   .product-mgmt-modal         — modal panel
  20  |  *   .product-mgmt-input         — form text/number inputs
  21  |  *   #product-field-sku          — SKU input
  22  |  *   #product-field-name         — name input
  23  |  *   #product-field-price        — price input
  24  |  */
  25  | 
  26  | test.describe('Product Management', () => {
  27  |   test.beforeEach(async ({ page }) => {
  28  |     await loginAs(page, 'admin', '9999');
  29  |     await selectWorkspace(page, WORKSPACES.INVENTORY);
  30  |   });
  31  | 
  32  |   // ── E2E-16: Assert product list loads ──────────────────────
  33  | 
  34  |   test('product list loads with at least 1 row', async ({ page }) => {
  35  |     // Wait for product management container.
  36  |     await page.waitForSelector('.product-mgmt', { timeout: 10_000 });
  37  | 
  38  |     // Product table must have at least 1 data row (dev-mock returns 18).
  39  |     const rows = page.locator('.product-mgmt-table tbody tr');
  40  |     await expect(rows.first()).toBeVisible({ timeout: 5_000 });
  41  | 
  42  |     const count = await rows.count();
  43  |     expect(count).toBeGreaterThanOrEqual(1);
  44  |   });
  45  | 
  46  |   // ── E2E-17: Assert product content is correct ──────────────
  47  | 
  48  |   test('product table contains expected mock products', async ({ page }) => {
  49  |     await page.waitForSelector('.product-mgmt', { timeout: 10_000 });
  50  | 
  51  |     // First product should be "Caffè Latte" (SKU: LATTE).
  52  |     const firstSku = page.locator('.product-mgmt-cell-sku').first();
  53  |     await expect(firstSku).toBeVisible({ timeout: 5_000 });
  54  |     await expect(firstSku).toHaveText('LATTE');
  55  | 
  56  |     // Table should contain product names from the mock.
  57  |     const tableText = await page.locator('.product-mgmt-table').textContent();
  58  |     expect(tableText).toContain('Latte');
  59  |     expect(tableText).toContain('Espresso');
  60  |   });
  61  | 
  62  |   // ── E2E-18: Open create product modal ─────────────────────
  63  | 
  64  |   test('opens create product modal with form fields', async ({ page }) => {
  65  |     await page.waitForSelector('.product-mgmt', { timeout: 10_000 });
  66  | 
  67  |     // Click "Add Product" button.
  68  |     const addBtn = page.locator('button:has-text("Add Product"), button:has-text("Tambah")');
  69  |     await addBtn.click();
  70  | 
  71  |     // Modal must appear.
  72  |     const modal = page.locator('.product-mgmt-overlay');
  73  |     await expect(modal).toBeVisible({ timeout: 5_000 });
  74  | 
  75  |     // Modal must contain form inputs: SKU, name, price.
  76  |     const skuInput = page.locator('#product-field-sku');
  77  |     const nameInput = page.locator('#product-field-name');
  78  |     const priceInput = page.locator('#product-field-price');
  79  | 
  80  |     await expect(skuInput).toBeVisible();
  81  |     await expect(nameInput).toBeVisible();
  82  |     await expect(priceInput).toBeVisible();
  83  | 
  84  |     // Cancel button must dismiss the modal.
  85  |     const cancelBtn = page.locator('button:has-text("Cancel"), button:has-text("Batal")');
  86  |     await cancelBtn.click();
  87  |     await expect(modal).not.toBeVisible({ timeout: 5_000 });
  88  |   });
  89  | 
  90  |   // ── Bonus: Edit product opens modal with pre-filled data ────
  91  | 
  92  |   test('edit product opens modal with pre-filled fields', async ({ page }) => {
  93  |     await page.waitForSelector('.product-mgmt', { timeout: 10_000 });
  94  | 
  95  |     // Wait for product table rows.
  96  |     const rows = page.locator('.product-mgmt-table tbody tr');
  97  |     await expect(rows.first()).toBeVisible({ timeout: 5_000 });
  98  | 
  99  |     // Click "Edit" on the first product row.
  100 |     const editBtn = page.locator('.product-mgmt-action-btn').filter({ hasText: 'Edit' }).first();
> 101 |     await editBtn.click();
      |                   ^ Error: locator.click: Test timeout of 30000ms exceeded.
  102 |     await page.waitForTimeout(500);
  103 | 
  104 |     // Edit modal must appear.
  105 |     await expect(page.locator('.product-mgmt-overlay')).toBeVisible({ timeout: 5_000 });
  106 | 
  107 |     // SKU field must be disabled (editing mode).
  108 |     const skuInput = page.locator('#product-field-sku');
  109 |     await expect(skuInput).toBeDisabled();
  110 | 
  111 |     // Name field must be pre-filled.
  112 |     const nameInput = page.locator('#product-field-name');
  113 |     const nameValue = await nameInput.inputValue();
  114 |     expect(nameValue.length).toBeGreaterThan(0);
  115 | 
  116 |     // Close the modal.
  117 |     await page.locator('button:has-text("Cancel"), button:has-text("Batal")').click();
  118 |     await expect(page.locator('.product-mgmt-overlay')).not.toBeVisible({ timeout: 5_000 });
  119 |   });
  120 | 
  121 |   // ── E2E-19: Create product form validation ────────────────
  122 | 
  123 |   test('create form shows disabled save when fields are empty', async ({ page }) => {
  124 |     await page.waitForSelector('.product-mgmt', { timeout: 10_000 });
  125 | 
  126 |     // Open create modal.
  127 |     const addBtn = page.locator('button:has-text("Add Product"), button:has-text("Tambah")');
  128 |     await addBtn.click();
  129 |     await expect(page.locator('.product-mgmt-overlay')).toBeVisible({ timeout: 5_000 });
  130 | 
  131 |     // Save/Create button must be disabled when SKU and name are empty.
  132 |     const saveBtn = page.locator(
  133 |       '.product-mgmt-modal-actions button:has-text("Create"), .product-mgmt-modal-actions button:has-text("Save")',
  134 |     ).first();
  135 | 
  136 |     // The button should be disabled (no SKU, no name).
  137 |     await expect(saveBtn).toBeDisabled({ timeout: 3_000 });
  138 | 
  139 |     // Fill in required fields.
  140 |     await page.locator('#product-field-sku').fill('TEST-SKU');
  141 |     await page.locator('#product-field-name').fill('Test Product');
  142 |     await page.locator('#product-field-price').fill('500');
  143 | 
  144 |     // Button should now be enabled.
  145 |     await expect(saveBtn).toBeEnabled({ timeout: 2_000 });
  146 |   });
  147 | });
  148 | 
```