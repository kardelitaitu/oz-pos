<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (0 findings) · scripts/generate-license-keys.{ps1,sh} verified; Dockerfile uses golang:1.25-alpine -> alpine:3.20; health.go (GET /api/health, returns status: ok) + healthcheck.go present; pb_schema.json present; go.mod is go 1.25.0; all build/deploy/env-var claims match the Go code -->

# OZ-POS License Server — Northflank Deployment Guide

> **ADR:** [`docs/decisions/2026-07-10-license-server.md`](../../docs/decisions/2026-07-10-license-server.md)
> **Repository:** `apps/license-server/`
> **Target host:** Northflank (Hobby tier, ~$6–12/month)
> **Last updated:** 2026-07-11

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Generate RSA Key Pair](#2-generate-rsa-key-pair)
3. [Build the Docker Image](#3-build-the-docker-image)
4. [Push to a Container Registry](#4-push-to-a-container-registry)
5. [Create the Northflank Service](#5-create-the-northflank-service)
6. [Attach Persistent Volume](#6-attach-persistent-volume)
7. [Set Environment Variables](#7-set-environment-variables)
8. [Import the Collections Schema](#8-import-the-collections-schema)
9. [Create the Admin User](#9-create-the-admin-user)
10. [Configure Custom Domain](#10-configure-custom-domain)
11. [Test the Endpoints](#11-test-the-endpoints)
12. [Ongoing Maintenance](#12-ongoing-maintenance)

---

## 1. Prerequisites

Before starting, ensure you have:

- [ ] A **Northflank account** — [Sign up at northflank.com](https://northflank.com/)
- [ ] **Docker** installed locally — [Get Docker](https://docs.docker.com/get-docker/)
- [ ] A **GitHub account** (or any container registry) to store the Docker image
- [ ] A **domain** (optional, e.g., `license.oz-pos.com`) for a custom URL
- [ ] **Go 1.25+** and **OpenSSL** installed locally (for key generation and testing)

---

## 2. Generate RSA Key Pair

The license server signs subscriptions with an RSA-2048 private key. The POS binary verifies them with the matching public key.

### 2.1 Generate the key pair

From the **repository root**, run the appropriate script for your OS:

```powershell
# Windows (PowerShell)
.\scripts\generate-license-keys.ps1
```

```bash
# Linux / macOS
bash scripts/generate-license-keys.sh
# Or: chmod +x scripts/generate-license-keys.sh && ./scripts/generate-license-keys.sh
```

This does:

1. Generates a `crates/oz-core/oz-license-private.pem` file (RSA-2048, PKCS8 PEM).
2. Extracts the public key into `crates/oz-core/oz-license.key.pub` (DER/SPKI format).
3. The private key file is **git-ignored** — never commit it.

### 2.2 Verify the keys exist

```
crates/oz-core/oz-license.key.pub       ← committed, embedded in the binary
crates/oz-core/oz-license-private.pem   ← git-ignored, loaded as env var on Northflank
```

### 2.3 Test locally (optional)

```bash
# Build the license server
cd apps/license-server
go build -o license-server .

# Run with the private key
$env:OZ_LICENSE_PRIVATE_KEY = (Get-Content -Raw ../../crates/oz-core/oz-license-private.pem)
./license-server serve --http=0.0.0.0:8080
```

---

## 3. Build the Docker Image

The `Dockerfile` uses a **multi-stage build**: `golang:1.25-alpine` compiles the binary, then copies it into `alpine:3.20` for a ~25 MB final image.

### 3.1 Build locally

```bash
docker build -t oz-pos/license-server -f apps/license-server/Dockerfile apps/license-server
```

### 3.2 Test the image locally

```bash
docker run --rm -p 8080:8080 \
  -v license_pb_data:/pb/pb_data \
  -e OZ_LICENSE_PRIVATE_KEY="$(Get-Content -Raw crates/oz-core/oz-license-private.pem)" \
  oz-pos/license-server
```

You should see:

```
RSA private key loaded successfully
[0.00ms] ... Server started at http://0.0.0.0:8080
```

Verify the health check:

```bash
curl http://localhost:8080/api/health
# → {"status":"ok","db_connected":true,"uptime_secs":42,"go_version":"go1.25","go_os":"linux","go_arch":"amd64"}
```

---

## 4. Push to a Container Registry

Northflank pulls from any public or private container registry. GitHub Container Registry (GHCR) is free with a GitHub account.

### 4.1 Tag and push to GHCR

```bash
# Tag with your GitHub username
docker tag oz-pos/license-server ghcr.io/YOUR_USERNAME/oz-pos-license-server:latest

# Login to GHCR
echo $GITHUB_TOKEN | docker login ghcr.io -u YOUR_USERNAME --password-stdin

# Push
docker push ghcr.io/YOUR_USERNAME/oz-pos-license-server:latest
```

> **Alternative registries:** Docker Hub, AWS ECR, Google Artifact Registry — all work. Northflank supports any registry with an authenticated URL.

---

## 5. Create the Northflank Service

### 5.1 Create a new project

1. Go to [Northflank Dashboard](https://app.northflank.com/).
2. Click **New Project** → name it `oz-pos-license`.
3. Select a region (pick one close to your users).

### 5.2 Create the Combined Service

1. In your project, click **Services** → **Create New Service**.
2. Choose **Combined Service**.
3. Under **Image Source**:
   - Select **External Registry**.
   - Enter the image URL: `ghcr.io/YOUR_USERNAME/oz-pos-license-server:latest`.
   - If using a private registry, add the credentials under **Registry Credentials**.

### 5.3 Configure the service

| Setting | Value |
|---|---|
| **Service Name** | `license-server` |
| **Port** | `8080` (HTTP) |
| **Public Access** | ✅ Enabled |
| **Compute Plan** | `nf-compute-10` (0.1 vCPU, 256 MB) — sufficient for the license server |

> 💡 **Pricing:** `nf-compute-10` is ~$2.70/month. The license server handles very low traffic (activation is a one-time event per customer).

### 5.4 Deploy

Click **Create & Deploy**. The service will start but **will crash** until you complete Step 6 (volume) and Step 7 (env var). That's expected.

---

## 6. Attach Persistent Volume

PocketBase stores its SQLite database and admin credentials in `/pb/pb_data`. This must survive container restarts.

1. **Stop the service** if it's running (it's crashing anyway, but ensure it's stopped).
2. Go to your service → **Volumes** tab.
3. Click **Create New Volume**.
   - **Name:** `pb-data`
   - **Type:** NVMe (faster, for a small SQLite file)
   - **Size:** 1 GB (more than enough for license management data)
   - **Mount Path:** `/pb/pb_data`
4. Click **Save**.

> 💡 **Pricing:** NVMe storage is $0.15/GB/month. A 1 GB volume costs ~$0.15/month.

---

## 7. Set Environment Variables

> **Going live?** The ordered, tick-box checklist of what is still missing (Brevo SMTP
> login id, verified sender, Paddle webhook secret) and the apply order is in
> [`go-live-checklist.md`](../../docs/operations/go-live-checklist.md). This section documents every variable
> in full.

The license server requires the RSA private key as an environment variable. **Never hardcode this in the Dockerfile or commit it.**

### 7.1 Create a Secret Group

1. Go to your project → **Secrets** tab.
2. Click **Create Secret Group** → name it `license-server-secrets`.
3. Add a secret:
   - **Key:** `OZ_LICENSE_PRIVATE_KEY`
   - **Value:** Paste the **entire** contents of `crates/oz-core/oz-license-private.pem` (including `-----BEGIN PRIVATE KEY-----` and `-----END PRIVATE KEY-----`).

   In PowerShell:

   ```powershell
   Get-Content -Raw crates/oz-core/oz-license-private.pem | Set-Clipboard
   ```

4. (Optional) Add the support-contact webhook:
   - **Key:** `OZ_DISCORD_WEBHOOK`
   - **Value:** The **Discord channel webhook URL** (Discord → channel → Settings → Integrations → Webhooks → New Webhook). This is what `/api/v1/web/contact` forwards website support-form messages to. **Never expose this URL to the browser** — the website only talks to the license server, which keeps the secret server-side. If it is unset, `/api/v1/web/contact` returns `503 not configured` and the website's contact form falls back to a mailto link.
5. Add the **OTP email sender** (required for the website dashboard login — without it `POST /api/v1/web/request-otp` returns `503 email delivery is not configured` and the login page shows its "not configured" state):
   - **Key:** `OZ_SMTP_HOST` — your relay's hostname
   - **Key:** `OZ_SMTP_PORT` — default `587` (TLS/STARTTLS) if unset
   - **Key:** `OZ_SMTP_USER` / `OZ_SMTP_PASSWORD` — credentials for the relay (omit for unauthenticated relays)
   - **Key:** `OZ_SMTP_FROM` — sender address. **Must be set explicitly and verified with your relay** — the code defaults to `no-reply@oz-pos.com`, which relays will reject or flag until that domain is yours. **Boot gate:** when `OZ_SMTP_HOST` is set, the server runs a sender-identity probe at startup (auth + `MAIL FROM` only — nothing is ever queued) and **fails fast** if `OZ_SMTP_FROM` is unset, is still the unowned default, or the relay permanently rejects it (e.g. Brevo `550 Sender address is not verified`). A transient relay outage only logs a warning, so a brief hiccup can't block a deploy. Unset `OZ_SMTP_HOST` skips the gate entirely (the endpoint answers 503 by design then).

   **No custom domain yet?** Northflank does **not** provide SMTP/email to apps — you need a third-party transactional relay, and `code.run` / `workers.dev` are not domains you can add DNS records to (no SPF/DKIM there). Until you own a domain, use a provider that works with a **verified sender email** instead:

   - **Brevo (current choice)** — SMTP login/username is a dedicated **SMTP login email** (Brevo → Settings → SMTP & API → copy the **Login** value; it is NOT your account email), password is the **SMTP key** (`xsmtpsib-…`). The `OZ_SMTP_FROM` address must be a **verified sender** in Brevo (Sender Identity → verify the email or domain) or sends fail. All three Brevo options work — **port 465 uses implicit TLS**, 587/2525 use STARTTLS (`smtp_mail.go` picks the transport by port). Example:
     ```
     OZ_SMTP_HOST=smtp-relay.brevo.com
     OZ_SMTP_PORT=587
     OZ_SMTP_USER=<brevo-smtp-login-email@smtp-brevo.com>
     OZ_SMTP_PASSWORD=xsmtpsib-…
     OZ_SMTP_FROM=<your-verified-sender@example.com>
     ```
   - **SendGrid (free tier)** — Settings → Sender Authentication → **Single Sender Verification** → verify the From address (e.g. your own email). SMTP: `smtp.sendgrid.net:587`, user `apikey`, password = your SendGrid API key, From = the verified sender.
   - **Amazon SES** — verify the sender **email address** (no domain required). Sandbox initially only delivers to verified recipients; request production access when live.

   > **Deliverability honesty:** without your own domain + SPF/DKIM/DMARC, inbox placement is best-effort — codes may land in spam. Once you own a domain: set `OZ_SMTP_FROM=noreply@<domain>`, add the provider's SPF include + DKIM records (and a DMARC policy), then the verified-sender fallback is no longer needed. This is the actual fix for "signup codes never land in spam".
6. (Optional) Web API CORS allowlist override:
   - **Key:** `OZ_WEB_ALLOWED_ORIGINS` — comma-separated origins allowed to call the web endpoints. **Defaults are already correct** for the current setup (`https://oz-pos.adikaradwiatmaja.workers.dev`, `https://oz-pos.com`, `http://localhost:4321`); only set this if you deploy the website to a different origin.
7. (Optional) Session lifetime override:
   - **Key:** `OZ_WEB_SESSION_TTL` — Go duration, default `24h` (e.g. `72h` to extend dashboard sessions).
8. Add the **Paddle webhook** secrets (required for the checkout → provisioning flow):
   - **Key:** `PADDLE_WEBHOOK_SECRET` — the endpoint secret key from Paddle → Developer tools → Notifications → Edit destination. Without it the webhook answers `503 not configured`. **Boot gate:** the server fails fast at startup if this (or `PADDLE_PRICE_TIERS`) is missing or malformed, so a misconfigured deploy can never silently answer 503/500 on every event.
   - **Key:** `PADDLE_PRICE_TIERS` — comma-separated `price_id:tier_key` pairs mapping every Paddle price to a tier, e.g. `pri_01h7abc123:pro,pri_01h7def456:premium`. Unmapped prices make provisioning fail with 500 (Paddle retries) until this is fixed. Copy the real price IDs from the Paddle dashboard (Catalog → Prices). Both vars are still read per-request, so a redeploy with fixed env recovers without a code change.
   - **Key:** `PADDLE_API_KEY` (optional) — server-side Paddle API key. Only needed when the customer email isn't passed in `custom_data` at checkout; the webhook falls back to fetching it via `GET /customers/{id}`.
   - **Key:** `PADDLE_API_URL` (optional) — defaults to `https://api.paddle.com`.
9. Click **Save**.

### 7.2 CORS for the website

The website is currently served from `https://oz-pos.adikaradwiatmaja.workers.dev` (until the `oz-pos.com` domain is bought) and calls the web endpoints (`/api/v1/web/contact`, `request-otp`, `verify-otp`, `/me`, `logout`) cross-origin.

- **Web OTP endpoints** enforce an **in-handler CORS allowlist** read from `OZ_WEB_ALLOWED_ORIGINS` (Step 6 above). Its default already includes the workers.dev origin, `oz-pos.com`, and `http://localhost:4321`, so **no configuration is needed** — just don't set the variable to an empty string, or the allowlist falls back to the default.
- **`/api/v1/web/contact`** relies on PocketBase's global CORS middleware, which allows all origins by default (stateless, no cookies). No configuration needed for the contact form to work. For hardening, restrict origins by adding the `--origins` flag to the `serve` command in the Dockerfile `CMD` (e.g. `--origins=https://oz-pos.adikaradwiatmaja.workers.dev,https://oz-pos.com,http://localhost:4321`).

### 7.3 Attach to the service

1. Go to your service → **Environment** tab.
2. Under **Secret Groups**, click **Attach**.
3. Select `license-server-secrets`.
4. Click **Save**.

### 7.4 Redeploy

Click **Redeploy** on the service. After deployment, the service should start without errors.

### 7.5 Configure the Paddle webhook

In the Paddle dashboard (**Developer tools → Notifications**):

1. Create a notification destination of type **URL (webhook)** pointing at `https://license.oz-pos.com/api/v1/paddle/webhook`.
2. Subscribe to the **Subscription** events: `subscription.created`, `subscription.activated`, `subscription.trialing`, `subscription.updated`, `subscription.canceled`, `subscription.paused`, `subscription.resumed`, `subscription.past_due` — plus `transaction.completed` / `transaction.payment_failed` (currently acknowledged and logged; one-time purchases only provision once a lifetime tier ships).
3. Copy the **endpoint secret key** into the `PADDLE_WEBHOOK_SECRET` secret (Step 8 in §7.1).
4. **Signature verification:** every request carries a `Paddle-Signature` header (`ts=<unix>;h1=<hex>`). The server verifies HMAC-SHA256 over `ts:rawBody` with the endpoint secret and rejects timestamps older than 5 minutes. Nothing else is trusted.
5. **Idempotency:** Paddle retries non-2xx responses; the server dedups by `event_id` (24h in-memory window) and upserts on `paddle_sub_id`, so replays are no-ops.
6. **Customer email:** the website checkout passes `custom_data.email` (the email the customer types on the pricing card), which the webhook reads to upsert the tenant — **no `PADDLE_API_KEY` needed**. `PADDLE_API_KEY` remains an optional fallback for events whose `custom_data` lacks the email.

---

## 8. Import the Collections Schema

PocketBase collections (`license_keys`, `tenants`, `subscriptions`, `tenant_machines`) are defined in `pb_schema.json`.

> ✅ **No action needed on a fresh deployment.** The server auto-imports the embedded `pb_schema.json` on first boot whenever any required collection is missing (see `ensureCollections` in `main.go`) — verified in the container log: `missing required collection "license_keys" — importing pb_schema.json`. The manual import below is only a fallback if you ever need to inspect or re-import by hand.

### 8.1 Via the Admin UI (Optional verification)

1. Navigate to your service's public URL: `https://<your-service>.code.run/_/`
2. Log in with the **admin user** created in Step 9 (you need to create it first).
3. Go to **Settings** → **Import Collections**.
4. Upload `apps/license-server/pb_schema.json`.
5. Click **Import**. All 4 collections should appear.

### 8.2 Via API (Alternative)

If you prefer automation, use PocketBase's collections API after creating the admin user:

```bash
# You'll need the admin token from Step 9
curl -X PUT https://<your-service>.code.run/api/collections/import \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d @apps/license-server/pb_schema.json
```

---

## 9. Create the Admin User

The PocketBase admin UI at `/_/` requires at least one superuser account. Create it via SSH.

### 9.1 Open the Northflank Shell

1. Go to your service → **Overview**.
2. Click **Shell** (opens an SSH session to the running container).

### 9.2 Create the superuser

```bash
/pb/pocketbase superuser upsert admin@oz-pos.com YOUR_STRONG_PASSWORD
```

> ⚠️ **Use a strong, unique password.** This account has full admin access to all license key data.

### 9.3 Verify

1. Navigate to `https://<your-service>.code.run/_/`.
2. Log in with `admin@oz-pos.com` and your password.
3. You should see the PocketBase admin dashboard.

---

## 10. Configure Custom Domain

Northflank provides a free `*.code.run` subdomain with auto-provisioned TLS. For production, configure a custom domain.

### 10.1 Add the domain

1. Go to your service → **Networking**.
2. Under **Custom Domains**, click **Add**.
3. Enter your domain: `license.oz-pos.com`.
4. Northflank provides the DNS target (a `code.run` subdomain).

### 10.2 Configure DNS

In your DNS provider (Cloudflare, Route53, etc.), add a **CNAME record**:

| Type | Name | Value | TTL |
|---|---|---|---|
| CNAME | `license` | `<target-from-northflank>.code.run` | Auto/300s |

Northflank automatically provisions a Let's Encrypt TLS certificate within a few minutes.

---

## 11. Test the Endpoints

### 11.1 Generate a test license key

1. Open the admin UI (`/_/`).
2. Go to the **license_keys** collection.
3. Click **New Record** and fill in:
   - `key`: `OZ-PRO-TEST-ABCD-EFGH-IJKL`
   - `tier_key`: `pro`
   - `max_stores`: `2`
   - `max_pos_instances`: `3`
   - `allowed_types`: `["restaurant-pos", "store-pos", "inventory", "admin"]`
   - `status`: `unused`
   - `expires_at`: A date **1 year from now**

### 11.2 Test the activation endpoint

```bash
curl -X POST https://license.oz-pos.com/api/v1/license/activate \
  -H "Content-Type: application/json" \
  -d '{
    "key": "OZ-PRO-TEST-ABCD-EFGH-IJKL",
    "tenant_id": "test-tenant-001",
    "machine_id": "machine-001",
    "business_name": "Test Cafe",
    "contact_name": "John Doe",
    "email": "john@testcafe.com"
  }'
```

**Expected response (200):**

```json
{
  "signed_payload": "{\"tenant_id\":\"test-tenant-001\",\"tier_key\":\"pro\",...}",
  "signature": "base64-encoded-rsa-signature...",
  "api_key": "oz_abc123..."
}
```

### 11.3 Test the status endpoint

`/status` is a **POST** endpoint authenticated with `Authorization: Bearer <api_key>` (the credential never appears in URLs, so it can't leak to access logs or Referer headers). Use the `api_key` returned by the activation call in §11.2:

```bash
curl -X POST https://license.oz-pos.com/api/v1/license/status \
  -H "Authorization: Bearer <api_key_from_activation>" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id": "test-tenant-001"}'
```

**Expected response (200):**

```json
{
  "tenant_id": "test-tenant-001",
  "tier_key": "pro",
  "status": "active",
  "max_stores": 2,
  "max_pos_instances": 3,
  "expires_at": "...",
  "grace_until": "..."
}
```

### 11.4 Test rate limiting

Send 6 activation requests in quick succession. The 6th should return **429 Too Many Requests**.

### 11.5 Test key brute-force protection

Send 3 invalid key attempts. The 4th should return **429 Too Many Requests** with a "too many attempts for this key" message.

### 11.6 Test the Paddle webhook

Send a **signed** `subscription.created` event (compute `Paddle-Signature: ts=<now>;h1=<hex HMAC-SHA256 of "ts:body" with the endpoint secret>`):

```bash
curl -X POST https://license.oz-pos.com/api/v1/paddle/webhook \
  -H "Paddle-Signature: ts=$(date +%s);h1=..." \
  -H "Content-Type: application/json" \
  -d '{"event_id":"evt_test_1","event_type":"subscription.created","data":{"id":"sub_test_1","status":"active","customer_id":"cus_test_1","custom_data":{"email":"buyer@test.com"},"items":[{"price":{"id":"<your_price_id>","product_id":"pro_1"},"quantity":1}],"current_billing_period":{"starts_at":"2026-08-16T00:00:00Z","ends_at":"2027-08-16T00:00:00Z"}}}'
```

The response must be **200** and a tenant + `OZ-PRO-...` license key + subscription must appear in the admin UI. An unsigned or tampered request must return **401** and create nothing.

---

## 12. Ongoing Maintenance

### Backup

Northflank provides **volume snapshots**. Enable them:

1. Go to **Volumes** → your `pb-data` volume.
2. Click **Backups** → **Create Backup Schedule**.
3. Set daily backups with 7-day retention.

Alternatively, export manually from the admin UI (`/_/` → **Settings** → **Export Collections**).

### Monitoring

- **Northflank Dashboard:** CPU, memory, and request logs are available in the service overview.
- **PocketBase Logs:** Viewable via the Shell (`less /pb/pb_data/logs.db`) or the admin UI.
- **Uptime Monitoring:** Add a health check endpoint monitor (e.g., UptimeRobot on `https://license.oz-pos.com/api/health`, which returns `{"status":"ok"}`). The payload also includes per-gate status objects: `smtp` (`configured`/`verified`/`error` — runtime sender-identity probe, re-run at most every 60s so monitors don't hammer the relay), `paddle` (`secret_configured`/`price_tiers_configured`/`price_tiers_mappings`/`error`), `rsa` (`configured`), and `discord` (`configured`). These are status, not liveness — only a DB outage fails the check. **Copy-paste monitor config (including the keyword monitor that alerts when `smtp.verified` flips to false): see [`uptime-monitor.md`](./uptime-monitor.md).**

### Updating the service

1. Make changes to `apps/license-server/`.
2. Rebuild the Docker image:

   ```bash
   docker build -t oz-pos/license-server -f apps/license-server/Dockerfile apps/license-server
   docker tag oz-pos/license-server ghcr.io/YOUR_USERNAME/oz-pos-license-server:latest
   docker push ghcr.io/YOUR_USERNAME/oz-pos-license-server:latest
   ```

3. In Northflank, go to your service → **Deployments** → **Redeploy**.
   - Northflank will pull the latest image and restart the container.
   - The persistent volume preserves all data across redeploys.

### Generating new license keys

1. Open the admin UI (`/_/`).
2. Go to **license_keys** → **New Record**.
3. Fill in the tier, quotas, allowed types, and set status to `unused`.
4. Send the key to the customer.

---

## Cost Summary

| Item | Monthly Cost |
|---|---|
| **Compute** (`nf-compute-10`, 256 MB) | ~$2.70 |
| **NVMe Storage** (1 GB) | ~$0.15 |
| **Data Transfer** (low — only activation calls) | ~$0 |
| **Custom Domain TLS** | Free |
| **Total** | **~$2.85/month** |

> Northflank's Sandbox tier includes 2 free services, so the license server may qualify for $0/month if it stays within Sandbox limits.

---

## Troubleshooting

| Issue | Solution |
|---|---|
| Service crashes immediately | Check logs: the most common cause is missing `OZ_LICENSE_PRIVATE_KEY` env var. |
| `OZ_LICENSE_PRIVATE_KEY environment variable is required` | The secret group is not attached or the env var name is misspelled. |
| `failed to decode PEM block` | The private key is not valid PEM. Ensure you pasted the entire file including `-----BEGIN`/`-----END-----`. |
| `failed to parse RSA private key` | The key format is wrong. Generate PKCS#8 using the script in Step 2. |
| Can't log into admin UI | Create the superuser via the Shell (Step 9). |
| Collections not showing | Shouldn't happen on fresh boots — the schema auto-imports on first boot. If collections are missing anyway (e.g. a partially-provisioned volume), import `pb_schema.json` via Settings → Import Collections (Step 8). |
| Rate limited in testing | Wait 1 hour for IP bucket to refill, or restart the container (rate limiter is in-memory). |
| Health check failing | The Go healthcheck binary pings `/api/health` with a 5s timeout. If the server is slow to start (e.g., first boot after volume attach), the container may flap as unhealthy for ~15s until PocketBase finishes initialisation. In the **unified image**, the shell healthcheck also fails the container after `OZ_HEALTH_SMTP_MAX_FAILS` (default 3) consecutive SMTP `verified:false` probes — check `docker inspect` → `State.Health` and the healthcheck stderr for `SMTP sender identity not verified`. Run `docker inspect` to check `State.Health`. |

> 💡 **Tip:** The Dockerfile healthcheck uses the standalone `/pb/healthcheck` Go binary (no curl dependency). It pings `/api/health` which returns `{"status":"ok"}` when PocketBase is healthy. The healthcheck was set up correctly in the Dockerfile.
