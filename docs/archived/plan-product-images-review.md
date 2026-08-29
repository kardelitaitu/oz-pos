# Product Image Storage Plan - Review Summary

## Review Date: 2026-08-21

## Executive Summary

The plan is **comprehensive and ready for implementation**. All key decisions have been made, technical details are specified, and the impact is minimal.

---

## ✅ Consistency Check

| Section | Free | Plus | Pro | Premium | Enterprise | Status |
|---------|------|------|-----|---------|------------|--------|
| Tier Limits | 200 | 500 | 2,000 | 5,000 | Unlimited | ✅ Consistent |
| Images Synced | No | Yes | Yes | Yes | Yes | ✅ Consistent |
| Server Storage | 0 | 6 MB | 24 MB | 60 MB | 6 GB max | ✅ Consistent |
| Success Metrics | 200 | 500 | 2K | 5K | — | ✅ Consistent |

---

## ✅ Technical Review

### Storage Estimation
- **Per image**: 512×512 WebP @ 40% quality = ~12 KB ✅
- **Realistic max**: 2.4 GB (40% of 6 GB volume) ✅
- **Free tier**: 0 GB (local only) ✅

### API Endpoints
1. **Upload** — POST /api/v1/product-images/{product_id} ✅
2. **Serve** — GET /api/v1/product-images/{product_id} ✅
3. **Delete** — DELETE /api/v1/product-images/{product_id} ✅
4. **Bulk Check** — POST /api/v1/product-images/bulk ✅

### Database Schema
- `product_images` table with proper indexes ✅
- `products.has_image` flag for sync ✅
- Unique constraint on (tenant_id, product_id) ✅

### Cleanup Logic
- Product deletion → delete image files ✅
- Image replacement → delete old, store new ✅
- Tenant deletion → delete entire directory ✅
- Orphaned file cleanup job ✅

---

## ✅ Code Changes Required

### Server-Side (Rust)
1. `crates/oz-core/src/subscription.rs` — Add `max_products()` and `supports_product_images()` ✅
2. `crates/oz-core/migrations/` — New migration for `product_images` table ✅
3. `apps/cloud-server/src/` — New image endpoints (upload, serve, delete, bulk) ✅
4. `crates/oz-core/src/db/products.rs` — Enforce product limits on create ✅

### Client-Side (TypeScript)
1. `ui/src/api/products.ts` — Add image upload/delete functions ✅
2. `ui/src/features/products/` — Update product card and edit dialog ✅
3. `platform/sync/src/transport.rs` — Add `has_image` to Product struct ✅

---

## ✅ Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Storage overflow | Low | High | Monitor usage, Enterprise per-tenant cap |
| Image upload abuse | Low | Medium | Rate limiting, file size limits |
| Orphaned files | Medium | Low | Daily cleanup job |
| Sync protocol bloat | Low | Medium | `has_image` flag, bulk check endpoint |

---

## ✅ Cost Impact

| Resource | Before | After | Delta | Status |
|----------|--------|-------|-------|--------|
| Storage | 0.02 GB | 0.15 GB | +0.13 GB | ✅ Negligible |
| CPU | 0.15 core | 0.152 core | +0.002 core | ✅ Negligible |
| Memory | 140 MB | 141 MB | +1 MB | ✅ Negligible |
| Terminals | 400 | 398 | -2 | ✅ Negligible |

---

## ✅ Success Criteria

- [ ] Free tier: 200 products, images local-only
- [ ] Plus tier: 500 products, images synced
- [ ] Pro tier: 2,000 products, images synced
- [ ] Premium tier: 5,000 products, images synced
- [ ] Enterprise: unlimited, images synced
- [ ] Storage stays under 6 GB
- [ ] No measurable performance impact
- [ ] Images deleted on product/tenant deletion

---

## Reviewer Notes

1. **Free tier local-only is smart** — Reduces server storage by 50%, clear upgrade path
2. **Client-side processing saves CPU** — Server just stores files, no image processing
3. **Tier limits on products, not images** — Simple, one number to remember
4. **Cleanup logic is thorough** — Covers all deletion scenarios
5. **Appendix A provides exact code changes** — Ready for implementation

---

## Recommendation

**APPROVE for implementation.** The plan is well-designed, technically sound, and has minimal impact on server resources. The Free tier local-only approach is particularly clever — it reduces costs while providing a clear upgrade incentive.

---

## Next Steps

1. Add `max_products()` to `SubscriptionTier` (Phase 1.1)
2. Create database migration (Phase 1.2)
3. Implement server endpoints (Phase 1.3-1.7)
4. Update client UI (Phase 2.1-2.6)
5. Add polish (Phase 3.1-3.4)
