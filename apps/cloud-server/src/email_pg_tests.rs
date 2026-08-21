use super::*;
use oz_core::export::email_report::SMTP_CONFIG_SETTINGS_KEY;

/// Integration test against a live Postgres instance — seeds a product,
/// a completed sale with lines, stock, and the settings the loop reads,
/// then exercises the whole analytics bundle + settings helpers on the
/// real database. Skips when Postgres is unreachable.
#[tokio::test]
async fn pg_integration_email_loop_reads_postgres() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());

    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG email-loop integration test skipped: {e}");
            return;
        }
    };

    let ns = format!("pg-email-test-{}", uuid::Uuid::now_v7());
    let product_id = format!("{ns}-product");
    let sku = format!("{ns}-sku");
    let category_id = format!("{ns}-cat");
    let sale_id = format!("{ns}-sale");
    let sale_line_id = format!("{ns}-line");
    // Use a fixed January date so parallel PG tests writing "today"
    // rows (webhooks, REST roundtrip) can never land inside this
    // test's analytics window.
    let now = "2026-01-15T09:00:00.000Z";

    // Clean up any leftovers from previous (failed) runs so assertions
    // count only this run's seeded rows.
    {
        let client = pool.get().await.unwrap();
        for sql in [
            "DELETE FROM sale_lines WHERE id LIKE 'pg-email-test-%'",
            "DELETE FROM sales WHERE id LIKE 'pg-email-test-%'",
            "DELETE FROM stock_summary WHERE item_id LIKE 'pg-email-test-%'",
            "DELETE FROM products WHERE id LIKE 'pg-email-test-%'",
            "DELETE FROM categories WHERE name LIKE 'pg-email-test-%'",
            "DELETE FROM settings WHERE key IN ('store.name', 'smtp_config', 'report_schedule', 'last_report_sent_at') AND value LIKE 'pg-email-test-%'",
        ] {
            client.execute(sql, &[]).await.unwrap();
        }
    }

    // ── Seed ──────────────────────────────────────────────────────
    let mut client = pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let category_name = format!("{ns} Cat");
    tx.execute(
        "INSERT INTO categories (id, name, colour, icon, created_at, updated_at) VALUES ($1, $2, '#fff', '', $3, $3)",
        &[&category_id, &category_name, &now],
    )
    .await
    .unwrap();
    let barcode = format!("{ns}-barcode");
    tx.execute(
        "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, \
         created_at, updated_at, price_updated_at, track_serial, product_type, version, \
         cost_minor, brand, rack_location, notes, unit, is_active, default_supplier_id, tenant_id)
         VALUES ($1, $2, 'Cold Brew', 5000, 'USD', $3, $4, $5, $5, $5, 0, 'retail', 1, 2000, NULL, NULL, NULL, NULL, 1, NULL, 'default')",
        &[&product_id, &sku, &category_id, &barcode, &now],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO stock_summary (item_id, location_id, qty, updated_at) VALUES ($1, $2, 4, $3)",
        &[
            &product_id,
            &oz_core::inventory::CANONICAL_DEFAULT_LOCATION_UUID,
            &now,
        ],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor, \
         discount_percent, discount_label, user_id, created_at, updated_at, subtotal_minor, \
         tax_total_minor, customer_id, version)
         VALUES ($1, 5000, 'USD', 1, 'completed', 'cash', 5000, 0, NULL, NULL, $2, $2, 5000, 0, NULL, 1)",
        &[&sale_id, &now],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, \
         tax_minor, tax_rate_id, serial_number, store_id, course, modifiers_json, tax_breakdown_json, cost_minor)
         VALUES ($1, $2, $3, 1, 5000, 5000, 'USD', 1, 0, NULL, NULL, NULL, NULL, NULL, NULL, 2000)",
        &[&sale_line_id, &sale_id, &sku],
    )
    .await
    .unwrap();
    tx.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES ($1, $2, $3)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at",
        &[&"store.name", &format!("{ns} Store"), &now],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // ── Exercise the analytics bundle ─────────────────────────────
    let config = ExportConfig {
        start_date: "2026-01-01".into(),
        end_date: "2026-01-31".into(),
        top_product_limit: 25,
        low_stock_threshold: 10,
    };
    let bundle = export_analytics_bundle_pg(&pool, config, "default", &format!("{ns} Store"))
        .await
        .unwrap();

    // Daily revenue: one completed sale of 5000 minor units.
    assert_eq!(bundle.daily_revenue.len(), 1);
    assert_eq!(bundle.daily_revenue[0].date, "2026-01-15");
    assert_eq!(bundle.daily_revenue[0].total_minor, 5000);
    assert_eq!(bundle.daily_revenue[0].sale_count, 1);
    assert_eq!(bundle.daily_revenue[0].cogs_minor, 2000);
    assert_eq!(bundle.daily_revenue[0].gross_profit_minor, 3000);

    // Weekly revenue: same sale in the week of 2026-01-12 (Monday).
    assert_eq!(bundle.weekly_revenue.len(), 1);
    assert_eq!(bundle.weekly_revenue[0].week_start, "2026-01-12");
    assert_eq!(bundle.weekly_revenue[0].total_minor, 5000);

    // Monthly revenue: 2026-01.
    assert_eq!(bundle.monthly_revenue.len(), 1);
    assert_eq!(bundle.monthly_revenue[0].month, "2026-01");
    assert_eq!(bundle.monthly_revenue[0].total_minor, 5000);

    // Top products: the seeded SKU with qty 1, revenue 5000, COGS 2000.
    assert_eq!(bundle.top_products.len(), 1);
    assert_eq!(bundle.top_products[0].sku, sku);
    assert_eq!(bundle.top_products[0].total_qty, 1);
    assert_eq!(bundle.top_products[0].total_minor, 5000);
    assert_eq!(bundle.top_products[0].cogs_minor, 2000);
    assert_eq!(bundle.top_products[0].gross_profit_minor, 3000);

    // Hourly heatmap: Sunday=0, the sale is at 09:00 UTC on a Thursday.
    assert!(
        bundle
            .hourly_heatmap
            .iter()
            .any(|h| h.day_of_week == 4 && h.hour == 9)
    );

    // Category breakdown: the seeded category, 5000 minor units.
    assert_eq!(bundle.category_breakdown.len(), 1);
    assert_eq!(
        bundle.category_breakdown[0].category_name,
        format!("{ns} Cat")
    );
    assert_eq!(bundle.category_breakdown[0].total_minor, 5000);
    assert_eq!(bundle.category_breakdown[0].percentage, 100.0);

    // Low stock: qty 4 ≤ threshold 10.
    assert!(
        bundle
            .low_stock_alerts
            .iter()
            .any(|a| a.sku == sku && a.current_qty == 4 && a.threshold == 10)
    );

    // Popularity + forecast are computed (may be empty without activity —
    // the typed bundle field is present by construction, so no claim is
    // made about row counts here).

    // ── Settings round-trip (SMTP config + schedule + dedup key) ──
    let smtp = SmtpConfig {
        host: "smtp.test.com".into(),
        port: 587,
        username: Some("u".into()),
        password: Some("pw".into()),
        from: "reports@test.com".into(),
        use_tls: true,
    };
    set_setting_pg(
        &pool,
        SMTP_CONFIG_SETTINGS_KEY,
        &serde_json::to_string(&smtp).unwrap(),
    )
    .await
    .unwrap();
    let loaded = get_smtp_config_pg(&pool, "default").await.unwrap().unwrap();
    assert_eq!(loaded.host, "smtp.test.com");
    assert_eq!(loaded.password, Some("pw".into()));

    let schedule = ReportScheduleConfig {
        enabled: true,
        cadence: "daily".into(),
        report_types: vec!["daily_revenue".into()],
        recipients: vec!["a@b.c".into()],
        send_at_time: "08:00".into(),
        timezone: "UTC".into(),
        lookback_days: 7,
    };
    set_setting_pg(
        &pool,
        REPORT_SCHEDULE_SETTINGS_KEY,
        &serde_json::to_string(&schedule).unwrap(),
    )
    .await
    .unwrap();
    let loaded = get_report_schedule_pg(&pool, "default")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded.cadence, "daily");
    assert_eq!(loaded.recipients, vec!["a@b.c".to_string()]);

    assert_eq!(
        get_store_name_pg(&pool, "default").await.unwrap(),
        format!("{ns} Store")
    );
    assert_eq!(get_setting_pg(&pool, LAST_SENT_KEY).await.unwrap(), None);

    // ── Shared SKU across tenants: joins must resolve within each
    //    sale's tenant, never cross into the other tenant's product ──
    let product_b_id = format!("{ns}-product-b");
    let sale_b_id = format!("{ns}-sale-b");
    let sale_b_line_id = format!("{ns}-line-b");
    let now_b = "2026-01-16T09:00:00.000Z";
    {
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        // Same SKU as tenant A's product, but owned by tenant-b with a
        // very different cost (9999 vs 2000) — if the COGS/popularity
        // joins were sku-only, A's rows would pick up B's cost.
        tx.execute(
            "INSERT INTO products (id, sku, name, price_minor, currency, category_id, barcode, \
             created_at, updated_at, price_updated_at, track_serial, product_type, version, \
             cost_minor, brand, rack_location, notes, unit, is_active, default_supplier_id, tenant_id)
             VALUES ($1, $2, 'Tenant B Cold Brew', 5000, 'USD', NULL, NULL, $3, $3, $3, 0, 'retail', 1, 9999, NULL, NULL, NULL, NULL, 1, NULL, 'tenant-b')",
            &[&product_b_id, &sku, &now_b],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sales (id, total_minor, currency, line_count, status, payment_method, tendered_minor, \
             discount_percent, discount_label, user_id, created_at, updated_at, subtotal_minor, \
             tax_total_minor, customer_id, version, tenant_id)
             VALUES ($1, 5000, 'USD', 1, 'completed', 'cash', 5000, 0, NULL, NULL, $2, $2, 5000, 0, NULL, 1, 'tenant-b')",
            &[&sale_b_id, &now_b],
        )
        .await
        .unwrap();
        tx.execute(
            "INSERT INTO sale_lines (id, sale_id, sku, qty, unit_minor, line_minor, currency, line_position, \
             tax_minor, tax_rate_id, serial_number, store_id, course, modifiers_json, tax_breakdown_json, cost_minor)
             VALUES ($1, $2, $3, 1, 5000, 5000, 'USD', 1, 0, NULL, NULL, NULL, NULL, NULL, NULL, NULL)",
            &[&sale_b_line_id, &sale_b_id, &sku],
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
    }

    // ── Tenant-filtered aggregation: each tenant's bundle sees ONLY its
    //    own sales, even when both tenants' sales sit in the same window
    //    and share a SKU (previously tenant_id only labelled metadata) ──
    let config2 = ExportConfig {
        start_date: "2026-01-01".into(),
        end_date: "2026-01-31".into(),
        top_product_limit: 25,
        low_stock_threshold: 10,
    };
    let bundle_default =
        export_analytics_bundle_pg(&pool, config2.clone(), "default", &format!("{ns} Store"))
            .await
            .unwrap();
    assert_eq!(
        bundle_default.daily_revenue.len(),
        1,
        "default must not see tenant-b's sale in its aggregation"
    );
    assert_eq!(
        (
            bundle_default.daily_revenue[0].total_minor,
            bundle_default.daily_revenue[0].cogs_minor
        ),
        (5000, 2000)
    );
    assert_eq!(bundle_default.top_products.len(), 1);
    assert_eq!(bundle_default.top_products[0].cogs_minor, 2000);

    let bundle_b = export_analytics_bundle_pg(&pool, config2, "tenant-b", "Tenant B Store")
        .await
        .unwrap();
    assert_eq!(
        bundle_b.daily_revenue.len(),
        1,
        "tenant-b must not see default's sale in its aggregation"
    );
    // B's day resolves B's product cost (9999) via the sale's tenant —
    // the join AND the WHERE filter both stay inside the tenant.
    assert_eq!(
        (
            bundle_b.daily_revenue[0].total_minor,
            bundle_b.daily_revenue[0].cogs_minor
        ),
        (5000, 9999)
    );
    assert_eq!(bundle_b.top_products.len(), 1);
    assert_eq!(bundle_b.top_products[0].cogs_minor, 9999);

    // ── Per-tenant scoped settings (suffix keys, bare-key fallback) ──
    set_setting_scoped_pg(&pool, "store.name", &format!("{ns} B Store"), "tenant-b")
        .await
        .unwrap();
    assert_eq!(
        get_store_name_pg(&pool, "tenant-b").await.unwrap(),
        format!("{ns} B Store"),
        "scoped key must win for tenant-b"
    );
    assert_eq!(
        get_store_name_pg(&pool, "default").await.unwrap(),
        format!("{ns} Store"),
        "default must keep reading the bare key"
    );
    // Remove the scoped key: the read falls back to the bare key.
    {
        let client = pool.get().await.unwrap();
        client
            .execute(
                "DELETE FROM settings WHERE key = $1",
                &[&scoped_key("store.name", "tenant-b")],
            )
            .await
            .unwrap();
    }
    assert_eq!(
        get_store_name_pg(&pool, "tenant-b").await.unwrap(),
        format!("{ns} Store"),
        "missing scoped key must fall back to the bare key"
    );

    // ── Loop decision paths (no SMTP involved) ──────────────────────
    // Tenant enumeration: `default` first, then the data-derived set
    // (offline_queue carries tenant-b).
    {
        let client = pool.get().await.unwrap();
        client
            .execute(
                "INSERT INTO offline_queue (id, action, payload, tenant_id)
                 VALUES ($1, 'test', '{}', 'tenant-b')",
                &[&format!("{ns}-queue")],
            )
            .await
            .unwrap();
    }
    let tenants = active_tenants_pg(&pool).await.unwrap();
    // The shared dev DB legitimately holds data-derived tenants from
    // other tests (e.g. the migration-bin integration test), so assert
    // the ordering properties — `default` first, this test's tenant
    // enumerated — not the exact list.
    assert_eq!(
        tenants.first(),
        Some(&"default".to_string()),
        "default must sort first in the tenant enumeration"
    );
    assert!(
        tenants.contains(&"tenant-b".to_string()),
        "tenant-b (data-derived via offline_queue) must be enumerated"
    );

    // Tenant-b has scoped SMTP + schedule + a future last-sent → not
    // due → the cycle returns Ok without attempting SMTP. `send_at_time`
    // is set 5 minutes ahead so the time-of-day gate is deterministic
    // regardless of when the suite runs.
    let send_at = (Utc::now() + chrono::Duration::minutes(5))
        .format("%H:%M")
        .to_string();
    let schedule_b = ReportScheduleConfig {
        enabled: true,
        cadence: "daily".into(),
        report_types: vec!["daily_revenue".into()],
        recipients: vec!["b@b.c".into()],
        send_at_time: send_at,
        timezone: "UTC".into(),
        lookback_days: 7,
    };
    set_setting_scoped_pg(
        &pool,
        SMTP_CONFIG_SETTINGS_KEY,
        &serde_json::to_string(&smtp).unwrap(),
        "tenant-b",
    )
    .await
    .unwrap();
    set_setting_scoped_pg(
        &pool,
        REPORT_SCHEDULE_SETTINGS_KEY,
        &serde_json::to_string(&schedule_b).unwrap(),
        "tenant-b",
    )
    .await
    .unwrap();
    set_setting_scoped_pg(&pool, LAST_SENT_KEY, "2099-01-01T00:00:00Z", "tenant-b")
        .await
        .unwrap();
    try_send_scheduled_for_tenant_pg(&pool, "tenant-b")
        .await
        .expect("already-sent tenant must short-circuit before SMTP");

    // A tenant with no scoped or bare config is skipped, not errored.
    try_send_scheduled_for_tenant_pg(&pool, "no-such-tenant")
        .await
        .expect("tenant without config must be skipped cleanly");

    // ── Cleanup (keys are namespaced; delete the seeded rows) ─────
    let client = pool.get().await.unwrap();
    for (sql, id) in [
        ("DELETE FROM sale_lines WHERE id = $1", &sale_line_id),
        ("DELETE FROM sales WHERE id = $1", &sale_id),
        ("DELETE FROM products WHERE id = $1", &product_id),
        ("DELETE FROM sale_lines WHERE id = $1", &sale_b_line_id),
        ("DELETE FROM sales WHERE id = $1", &sale_b_id),
        ("DELETE FROM products WHERE id = $1", &product_b_id),
        ("DELETE FROM categories WHERE id = $1", &category_id),
    ] {
        client.execute(sql, &[&id]).await.unwrap();
    }
    client
        .execute(
            "DELETE FROM settings WHERE key IN ('store.name', 'smtp_config', 'report_schedule', \
             'last_report_sent_at', 'store.name:tenant-b', 'smtp_config:tenant-b', \
             'report_schedule:tenant-b', 'last_report_sent_at:tenant-b')",
            &[],
        )
        .await
        .unwrap();
    client
        .execute(
            "DELETE FROM offline_queue WHERE id = $1",
            &[&format!("{ns}-queue")],
        )
        .await
        .unwrap();
}

// ── sent_reports dedup (at-most-once send) ─────────────────────

/// The dedup key must be stable per cadence: daily/weekly → the day
/// (weekly on its Monday), monthly → the month — so a crash + retry
/// always recomputes the same key for the same scheduled slot.
#[test]
fn period_for_schedule_buckets_by_cadence() {
    // 2026-01-15 is a Thursday; the Monday of that week is 2026-01-12.
    let now_tz = chrono::DateTime::parse_from_rfc3339("2026-01-15T09:00:00+00:00").unwrap();
    let base = || ReportScheduleConfig {
        enabled: true,
        cadence: "daily".into(),
        report_types: vec!["daily_revenue".into()],
        recipients: vec!["a@b.c".into()],
        send_at_time: "08:00".into(),
        timezone: "UTC".into(),
        lookback_days: 7,
    };

    let mut daily = base();
    daily.cadence = "daily".into();
    assert_eq!(period_for_schedule(&daily, now_tz), "2026-01-15");

    let mut weekly = base();
    weekly.cadence = "weekly".into();
    assert_eq!(period_for_schedule(&weekly, now_tz), "2026-01-12");

    let mut monthly = base();
    monthly.cadence = "monthly".into();
    assert_eq!(period_for_schedule(&monthly, now_tz), "2026-01");
}

/// Claim/release semantics against a live Postgres: the first claim
/// wins, a second claim for the same period loses (that is the
/// crash-recovery dedup), a different period is independent, and
/// releasing a failed claim lets the period retry.
#[tokio::test]
async fn pg_integration_sent_reports_claim_release() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG sent_reports integration test skipped: {e}");
            return;
        }
    };
    let tenant = format!("pg-sr-test-{}", uuid::Uuid::now_v7());
    let period = "2026-01-15";
    {
        let client = pool.get().await.unwrap();
        client
            .execute("DELETE FROM sent_reports WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
    }

    // First claim wins.
    assert!(
        claim_period_pg(&pool, &tenant, period, "id-1")
            .await
            .unwrap()
    );
    // The same period is already claimed — a restart or racing
    // instance must skip it.
    assert!(
        !claim_period_pg(&pool, &tenant, period, "id-2")
            .await
            .unwrap()
    );
    // A different period is independent.
    assert!(
        claim_period_pg(&pool, &tenant, "2026-01-16", "id-3")
            .await
            .unwrap()
    );
    // Releasing a failed claim lets the period retry.
    release_period_pg(&pool, &tenant, period).await.unwrap();
    assert!(
        claim_period_pg(&pool, &tenant, period, "id-4")
            .await
            .unwrap()
    );

    let client = pool.get().await.unwrap();
    client
        .execute("DELETE FROM sent_reports WHERE tenant_id = $1", &[&tenant])
        .await
        .unwrap();
}

/// The loop must skip an already-claimed period BEFORE sending: with a
/// due schedule (send_at_time = now) and the period pre-claimed, the
/// inner cycle returns Ok without ever attempting SMTP — proving a
/// crash after a successful send can never re-send the report.
#[tokio::test]
async fn pg_integration_sent_reports_skips_claimed_period_before_smtp() {
    let url = std::env::var("OZ_TEST_PG_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:15432/postgres".into());
    let pool = match crate::db::DbPool::connect_postgres(&url, false, 20, true).await {
        Ok(crate::db::DbPool::Postgres(pool)) => pool,
        Ok(_) => unreachable!("connect_postgres with a postgres:// URL returns Postgres"),
        Err(e) => {
            eprintln!("PG sent_reports integration test skipped: {e}");
            return;
        }
    };
    let ns = format!("pg-sr-skip-{}", uuid::Uuid::now_v7());
    let tenant = format!("{ns}-t");
    let now = Utc::now();
    // A due schedule: send_at_time is the current minute, so the
    // scheduler's 2-minute time-of-day gate passes deterministically.
    let schedule = ReportScheduleConfig {
        enabled: true,
        cadence: "daily".into(),
        report_types: vec!["daily_revenue".into()],
        recipients: vec!["x@y.z".into()],
        send_at_time: now.format("%H:%M").to_string(),
        timezone: "UTC".into(),
        lookback_days: 7,
    };
    let period = period_for_schedule(&schedule, resolve_now_in_timezone(&schedule.timezone));
    {
        let client = pool.get().await.unwrap();
        client
            .execute("DELETE FROM sent_reports WHERE tenant_id = $1", &[&tenant])
            .await
            .unwrap();
        // Scoped SMTP + schedule, no last-sent, and the period already
        // claimed (as if a crashed earlier attempt had sent it).
        for (key, value) in [
            (
                scoped_key(SMTP_CONFIG_SETTINGS_KEY, &tenant),
                serde_json::to_string(&SmtpConfig {
                    host: "smtp.invalid".into(),
                    port: 25,
                    username: None,
                    password: None,
                    from: "r@x.yz".into(),
                    use_tls: false,
                })
                .unwrap(),
            ),
            (
                scoped_key(REPORT_SCHEDULE_SETTINGS_KEY, &tenant),
                serde_json::to_string(&schedule).unwrap(),
            ),
        ] {
            set_setting_pg(&pool, &key, &value).await.unwrap();
        }
        claim_period_pg(&pool, &tenant, &period, "crashed-attempt")
            .await
            .unwrap();
    }

    // If the dedup fails, this would attempt SMTP to smtp.invalid and
    // return Err — Ok proves the skip happened before any send.
    try_send_scheduled_tenant_inner_pg(&pool, &tenant)
        .await
        .expect("already-claimed period must skip before SMTP");

    // Cleanup.
    let client = pool.get().await.unwrap();
    for (sql, key) in [
        (
            "DELETE FROM settings WHERE key = $1",
            scoped_key(SMTP_CONFIG_SETTINGS_KEY, &tenant),
        ),
        (
            "DELETE FROM settings WHERE key = $1",
            scoped_key(REPORT_SCHEDULE_SETTINGS_KEY, &tenant),
        ),
    ] {
        client.execute(sql, &[&key]).await.unwrap();
    }
    client
        .execute("DELETE FROM sent_reports WHERE tenant_id = $1", &[&tenant])
        .await
        .unwrap();
}
