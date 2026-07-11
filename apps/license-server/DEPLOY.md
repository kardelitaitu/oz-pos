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
- [ ] **Go 1.24+** and **OpenSSL** installed locally (for key generation and testing)

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

The `Dockerfile` uses a **multi-stage build**: `golang:1.24-alpine` compiles the binary, then copies it into `alpine:3.20` for a ~25 MB final image.

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
# → "OK" (or PocketBase health response)
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

4. Click **Save**.

### 7.2 Attach to the service

1. Go to your service → **Environment** tab.
2. Under **Secret Groups**, click **Attach**.
3. Select `license-server-secrets`.
4. Click **Save**.

### 7.3 Redeploy

Click **Redeploy** on the service. After deployment, the service should start without errors.

---

## 8. Import the Collections Schema

PocketBase collections (`license_keys`, `tenants`, `subscriptions`, `tenant_machines`) are defined in `pb_schema.json`. They need to be imported into the running instance.

### 8.1 Via the Admin UI (Recommended)

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

```bash
curl https://license.oz-pos.com/api/v1/license/status/test-tenant-001
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
- **Uptime Monitoring:** Add a health check endpoint monitor (e.g., UptimeRobot on `https://license.oz-pos.com/api/v1/license/status/_health`).

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
| Collections not showing | Import `pb_schema.json` via Settings → Import Collections (Step 8). |
| Rate limited in testing | Wait 1 hour for IP bucket to refill, or restart the container (rate limiter is in-memory). |
| Health check failing | PocketBase doesn't expose `/api/health` by default. The Dockerfile healthcheck may need adjustment — check service logs. |

> 💡 **Tip:** The Dockerfile healthcheck uses `curl -f http://localhost:8080/api/` — PocketBase always responds on `/api/` even before collections are loaded. This was set up correctly in the Dockerfile.
