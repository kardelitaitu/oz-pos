#!/usr/bin/env python3
"""End-to-end verification of the Paddle webhook against sandbox-style events.

Drives a LOCAL license server (the real binary) with realistic Paddle
Billing sandbox payloads — correct Paddle-Signature HMAC, exact entity
shapes — and exercises the whole purchase chain without a Paddle account:

  subscription.created (signed)  → tenant + license key + subscription
  receipt email (SMTP sink)      → the license key is emailed to the buyer
  request-otp / verify-otp       → dashboard session (code read from sink)
  GET /me                        → subscription visible in the account
  POST /activate (key, no key)   → api_key minted, webhook sub reused
  POST /status (Bearer api_key)  → POS sees the active subscription
  subscription.updated           → tier/expiry refreshed (and /status sees it)
  subscription.canceled          → subscription leaves the active set
  replay of the same event_id    → 200 duplicate (no double provisioning)
  tampered signature             → 401

Usage (from repo root, with scripts/dev-smtp-sink.py + the license server
already running):

  python scripts/e2e-paddle-webhook.py [SMTP_LOG]

Env overrides:
  LICENSE_URL      base URL of the license server (default http://127.0.0.1:8090)
  WEBHOOK_SECRET   Paddle webhook secret the server was started with
                   (default e2e-sandbox-secret)
  SMTP_LOG         sink log file (default .e2e-smtp.log, or argv[1])
"""
import hashlib
import hmac
import json
import os
import re
import sys
import time
import urllib.request
import urllib.error

LICENSE_URL = os.environ.get("LICENSE_URL", "http://127.0.0.1:8090").rstrip("/")
WEBHOOK_SECRET = os.environ.get("WEBHOOK_SECRET", "e2e-sandbox-secret")
SMTP_LOG = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SMTP_LOG", ".e2e-smtp.log")
WEBHOOK = LICENSE_URL + "/api/v1/paddle/webhook"

# Paddle sandbox price ids mapped by the server's PADDLE_PRICE_TIERS.
PRICE_PRO = "pri_test_sandbox_pro"
PRICE_PREMIUM = "pri_test_sandbox_premium"

BUYER_EMAIL = "buyer@sandbox.test"
SUB_ID = "sub_01h7sandbox000001"
CUSTOMER_ID = "ctm_01h7sandbox000001"

PASS = 0
FAIL = 0


def check(name, cond, detail=""):
    global PASS, FAIL
    if cond:
        PASS += 1
        print(f"  PASS  {name}" + (f"  ({detail})" if detail else ""))
    else:
        FAIL += 1
        print(f"  FAIL  {name}" + (f"  ({detail})" if detail else ""))


def sign(body: str, ts: int, secret: str = WEBHOOK_SECRET) -> str:
    mac = hmac.new(secret.encode(), f"{ts}:{body}".encode(), hashlib.sha256)
    return f"ts={ts};h1={mac.hexdigest()}"


def post(path: str, body: str, signature: str | None = None) -> tuple[int, str]:
    req = urllib.request.Request(LICENSE_URL + path, data=body.encode(), method="POST")
    req.add_header("Content-Type", "application/json")
    if signature is not None:
        req.add_header("Paddle-Signature", signature)
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return resp.status, resp.read().decode()
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode()


def send_webhook(body: str, ts: int | None = None, secret: str | None = None) -> tuple[int, str]:
    ts = ts if ts is not None else int(time.time())
    return post("/api/v1/paddle/webhook", body, sign(body, ts, secret or WEBHOOK_SECRET))


def sink_scan(pattern: str, required: bool = True) -> str | None:
    """Scan the sink log for the newest message block matching pattern."""
    try:
        with open(SMTP_LOG, encoding="utf-8", errors="replace") as fh:
            text = fh.read()
    except OSError:
        text = ""
    # Walk blocks from the end so we grab the most recent match.
    for block in reversed(re.split(r"----- BEGIN MESSAGE -----", text)):
        m = re.search(pattern, block)
        if m:
            return m.group(1)
    if required:
        print(f"  (could not find {pattern!r} in {SMTP_LOG})")
    return None


def created_body() -> str:
    return json.dumps({
        "event_id": "evt_01h7sandboxcreated",
        "event_type": "subscription.created",
        "occurred_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime()),
        "notification_id": "ntf_01h7sandboxcreated",
        "data": {
            "id": SUB_ID,
            "status": "active",
            "customer_id": CUSTOMER_ID,
            "currency_code": "USD",
            "created_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime()),
            "custom_data": {"email": BUYER_EMAIL},
            "items": [{
                "price": {"id": PRICE_PRO, "product_id": "pro_01h7sandbox"},
                "quantity": 1,
                "status": "active",
            }],
            "current_billing_period": {
                "starts_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime()),
                "ends_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime(time.time() + 2592000)),
            },
        },
    })


def updated_body() -> str:
    return json.dumps({
        "event_id": "evt_01h7sandboxupdated",
        "event_type": "subscription.updated",
        "occurred_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime()),
        "data": {
            "id": SUB_ID,
            "status": "active",
            "customer_id": CUSTOMER_ID,
            "currency_code": "USD",
            "custom_data": {"email": BUYER_EMAIL},
            "items": [{
                "price": {"id": PRICE_PREMIUM, "product_id": "pro_01h7sandbox"},
                "quantity": 1,
                "status": "active",
            }],
            "current_billing_period": {
                "starts_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime()),
                "ends_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime(time.time() + 7776000)),
            },
        },
    })


def canceled_body() -> str:
    return json.dumps({
        "event_id": "evt_01h7sandboxcanceled",
        "event_type": "subscription.canceled",
        "occurred_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime()),
        "data": {
            "id": SUB_ID,
            "status": "canceled",
            "customer_id": CUSTOMER_ID,
            "currency_code": "USD",
            "custom_data": {"email": BUYER_EMAIL},
            "items": [{
                "price": {"id": PRICE_PREMIUM, "product_id": "pro_01h7sandbox"},
                "quantity": 1,
                "status": "active",
            }],
            "scheduled_change": {
                "effective_at": time.strftime("%Y-%m-%dT%H:%M:%S.000Z", time.gmtime(time.time() + 7776000)),
                "status": "active",
            },
        },
    })


def main():
    print("== Paddle webhook E2E (sandbox-style events) ==")
    print(f"server: {LICENSE_URL}  sink log: {SMTP_LOG}")
    if not os.path.exists(SMTP_LOG):
        print(f"ERROR: SMTP log {SMTP_LOG} not found — is scripts/dev-smtp-sink.py running?")
        sys.exit(1)

    # ── 1. Tampered signature rejected ────────────────────────────
    print("\n[1] signature gate")
    code, _ = post("/api/v1/paddle/webhook", created_body(), "ts=1;h1=deadbeef")
    check("tampered signature -> 401", code == 401, f"got {code}")

    # ── 2. subscription.created provisions everything ─────────────
    print("\n[2] subscription.created")
    code, body = send_webhook(created_body())
    check("webhook -> 200", code == 200, f"got {code}: {body[:120]}")
    key = sink_scan(r"Your license key is:\s*\n\s*\n([A-Z0-9-]+)")
    check("receipt email carried the license key", bool(key) and key.startswith("OZ-"), key or "")

    # ── 3. Dashboard login (OTP) → /me shows the subscription ─────
    print("\n[3] dashboard login via OTP")
    code, body = post("/api/v1/web/request-otp", json.dumps({"email": BUYER_EMAIL}))
    check("request-otp -> 200", code == 200, f"got {code}")
    otp = sink_scan(r"verification code is: (\d{6})")
    check("OTP emailed to buyer", bool(otp), otp or "")
    code, body = post("/api/v1/web/verify-otp", json.dumps({"email": BUYER_EMAIL, "code": otp}))
    token = ""
    if code == 200:
        token = json.loads(body).get("token", "")
    check("verify-otp -> session token", code == 200 and len(token) > 20, f"got {code}")
    req = urllib.request.Request(LICENSE_URL + "/api/v1/web/me", headers={"Authorization": f"Bearer {token}"})
    me = {}
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            me = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  (me failed: {e.code})")
    sub = me.get("subscription") or {}
    lic = me.get("license") or {}
    check("/me tenant email", me.get("tenant", {}).get("email") == BUYER_EMAIL)
    check("/me subscription tier=pro active", sub.get("tierKey") == "pro" and sub.get("status") == "active",
          json.dumps(sub)[:120])
    # Pre-activation the account page shows the subscription (the key is
    # emailed); the license block appears once the POS activates the key
    # (activated_by set) — asserted again after step 5.

    # ── 4. POS activation with the emailed key (no api_key) ───────
    print("\n[4] POS activation")
    code, body = post("/api/v1/license/activate", json.dumps({
        "key": key,
        "machine_id": "0123456789abcde",
        "email": BUYER_EMAIL,
    }))
    api_key = ""
    if code == 200:
        api_key = json.loads(body).get("api_key", "")
    check("activate (email+key, no api_key) -> 200 + api_key", code == 200 and api_key.startswith("oz_"),
          f"got {code}")

    # ── 5. /status with the minted api_key ────────────────────────
    print("\n[5] POS /status")
    req = urllib.request.Request(LICENSE_URL + "/api/v1/license/status",
                                 data=json.dumps({"tenant_id": me.get("tenant", {}).get("id")}).encode(),
                                 method="POST",
                                 headers={"Content-Type": "application/json",
                                          "Authorization": f"Bearer {api_key}"})
    status = {}
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            status = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  (status failed: {e.code})")
    check("/status active tier=pro", status.get("active") is True and status.get("tier") == "pro",
          json.dumps(status)[:120])

    # After activation the dashboard must show the SAME key that was
    # emailed (activated_by is now set on the license_keys record).
    req = urllib.request.Request(LICENSE_URL + "/api/v1/web/me", headers={"Authorization": f"Bearer {token}"})
    me_after = {}
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            me_after = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  (me failed: {e.code})")
    check("/me license key matches receipt after activation",
          (me_after.get("license") or {}).get("key") == key)

    # ── 6. subscription.updated (renewal: pro -> premium) ─────────
    print("\n[6] subscription.updated (renewal)")
    code, body = send_webhook(updated_body())
    check("updated -> 200", code == 200, f"got {code}")
    req = urllib.request.Request(LICENSE_URL + "/api/v1/license/status",
                                 data=json.dumps({"tenant_id": me.get("tenant", {}).get("id")}).encode(),
                                 method="POST",
                                 headers={"Content-Type": "application/json",
                                          "Authorization": f"Bearer {api_key}"})
    status = {}
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            status = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  (status failed: {e.code})")
    check("/status now tier=premium", status.get("tier") == "premium", json.dumps(status)[:120])

    # ── 7. subscription.canceled → leaves the active set ──────────
    print("\n[7] subscription.canceled")
    code, body = send_webhook(canceled_body())
    check("canceled -> 200", code == 200, f"got {code}")
    req = urllib.request.Request(LICENSE_URL + "/api/v1/license/status",
                                 data=json.dumps({"tenant_id": me.get("tenant", {}).get("id")}).encode(),
                                 method="POST",
                                 headers={"Content-Type": "application/json",
                                          "Authorization": f"Bearer {api_key}"})
    status = {}
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            status = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  (status failed: {e.code})")
    check("/status active=false after cancel", status.get("active") is False, json.dumps(status)[:120])

    # ── 8. Replay of the created event → no-op ────────────────────
    print("\n[8] replay / idempotency")
    code, body = send_webhook(created_body())
    check("replay -> 200 duplicate", code == 200 and "duplicate" in body, f"got {code}: {body[:80]}")
    code, body = post("/api/v1/web/request-otp", json.dumps({"email": BUYER_EMAIL}))
    # A duplicate subscription.created must NOT have minted a second key —
    # re-request the OTP and confirm /me still shows the SAME key.
    otp = sink_scan(r"verification code is: (\d{6})")
    code, body = post("/api/v1/web/verify-otp", json.dumps({"email": BUYER_EMAIL, "code": otp}))
    token = json.loads(body).get("token", "")
    req = urllib.request.Request(LICENSE_URL + "/api/v1/web/me", headers={"Authorization": f"Bearer {token}"})
    me2 = {}
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            me2 = json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        print(f"  (me failed: {e.code})")
    check("no duplicate key after replay", (me2.get("license") or {}).get("key") == key)

    print(f"\n== RESULT: {PASS} passed, {FAIL} failed ==")
    sys.exit(1 if FAIL else 0)


if __name__ == "__main__":
    main()
