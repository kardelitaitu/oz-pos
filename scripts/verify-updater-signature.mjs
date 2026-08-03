#!/usr/bin/env node
// ── OZ-POS Updater Signature Verifier (AUDIT-28 RELEASE-04/06) ────────
//
// Verifies that an installer/update asset matches the Ed25519 signature
// recorded in a Tauri updater `latest.json` manifest, using the public key
// embedded in tauri.conf.json::plugins.updater.pubkey (minisign-style
// base64). This is the post-build check the release workflow runs before
// publishing: a release must never go live with a signature that does not
// verify against the committed pubkey.
//
// Usage:
//   node scripts/verify-updater-signature.mjs <manifest.json> <platform> <installer-path>
//   node scripts/verify-updater-signature.mjs --self-test
//
// Exit codes:
//   0 — signature verified
//   1 — verification failed
//   2 — usage error
//
// The signature is verified over the RAW installer bytes (matching
// `tauri signer sign`), not over a hash.
//
// NOTE: uses node:crypto (SPKI DER) rather than crypto.subtle — Node 24's
// WebCrypto rejects raw Ed25519 key imports with the `sign` usage, so the
// low-level API is the deterministic path across Node versions.

import { createPrivateKey, createPublicKey, sign, verify } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");

// SPKI DER prefix (RFC 8410): SEQUENCE(30 2a) { AlgId Ed25519, BIT STRING pubkey }
const SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

function decodeMinisignPubkey(pubkeyB64) {
  const buf = Buffer.from(pubkeyB64, "base64");
  const text = buf.toString("utf8");
  if (text.includes("untrusted comment") || text.includes("\n")) {
    const lines = text
      .split("\n")
      .map((l) => l.trim())
      .filter(Boolean);
    const keyLine = lines[lines.length - 1];
    const key = Buffer.from(keyLine, "base64");
    // minisign Ed25519 keys carry a 0x45 ('E') tag byte before the 32-byte key
    if (key.length === 33 && key[0] === 0x45) return key.subarray(1);
    return key;
  }
  if (buf.length === 33 && buf[0] === 0x45) return buf.subarray(1);
  return buf;
}

function verifyBytes(pubkeyRaw, bytes, signature) {
  const key = createPublicKey({
    key: Buffer.concat([SPKI_PREFIX, pubkeyRaw]),
    format: "der",
    type: "spki",
  });
  return verify(null, bytes, key, signature);
}

function verifyManifest({ manifestPath, platform, installerRead, pubkeyB64 }) {
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  } catch (e) {
    console.error(`Failed to read manifest: ${e.message}`);
    process.exit(1);
  }
  const entry = manifest.platforms?.[platform];
  if (!entry || !entry.signature) {
    console.error(`Manifest has no signature for platform '${platform}'`);
    process.exit(1);
  }
  let bytes;
  try {
    bytes = installerRead();
  } catch (e) {
    console.error(`Failed to read installer: ${e.message}`);
    process.exit(1);
  }
  const pubkeyRaw = decodeMinisignPubkey(pubkeyB64);
  if (pubkeyRaw.length !== 32) {
    console.error(`Invalid pubkey length: ${pubkeyRaw.length} (expected 32)`);
    process.exit(1);
  }
  return verifyBytes(pubkeyRaw, bytes, Buffer.from(entry.signature, "base64"));
}

function selfTest() {
  // Deterministic test keypair: seed = 32x 0x42; build a minisign-style
  // pubkey from the SPKI-derived raw key, then verify sign/verify round-trip
  // and tamper rejection.
  const PKCS8_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");
  const seed = Buffer.alloc(32, 0x42);
  const priv = createPrivateKey({
    key: Buffer.concat([PKCS8_PREFIX, seed]),
    format: "der",
    type: "pkcs8",
  });
  const jwk = priv.export({ format: "jwk" });
  const pubkeyRaw = Buffer.from(jwk.x, "base64url");
  const tagged = Buffer.concat([Buffer.from([0x45]), pubkeyRaw]);
  const minisignPubkey = Buffer.from(
    `untrusted comment: minisign public key: TEST\n${tagged
      .toString("base64")
      .replace(/(.{64})/g, "$1\n")}`,
    "utf8"
  ).toString("base64");

  const payload = Buffer.from("oz-pos signature verifier self-test", "utf8");
  const signature = sign(null, payload, priv);

  // The minisign-style pubkey must decode back to the derived 32-byte key.
  const decoded = decodeMinisignPubkey(minisignPubkey);
  if (!decoded.equals(pubkeyRaw)) {
    console.error("FAIL minisign pubkey decode round-trip");
    process.exit(1);
  }

  const ok = verifyBytes(decoded, payload, signature);
  if (!ok) {
    console.error("FAIL signature verification");
    process.exit(1);
  }

  const tampered = verifyBytes(decoded, Buffer.concat([payload, Buffer.from([0])]), signature);
  if (tampered) {
    console.error("FAIL tampered payload accepted");
    process.exit(1);
  }

  console.log("self-test: signature verification (incl. tamper rejection) PASSED");
  process.exit(0);
}

const args = process.argv.slice(2);
if (args.includes("--self-test")) {
  selfTest();
}

const [manifestPath, platform, installerPath] = args;
if (!manifestPath || !platform || !installerPath) {
  console.error(
    "Usage: node scripts/verify-updater-signature.mjs <manifest.json> <platform> <installer-path>"
  );
  process.exit(2);
}

const pubkey = JSON.parse(
  readFileSync(join(ROOT, "apps/desktop-client/tauri.conf.json"), "utf8")
).plugins.updater.pubkey;
if (!pubkey) {
  console.error("tauri.conf.json::plugins.updater.pubkey is missing");
  process.exit(1);
}

const ok = verifyManifest({
  manifestPath,
  platform,
  installerRead: () => readFileSync(installerPath),
  pubkeyB64: pubkey,
});
if (!ok) {
  console.error(`SIGNATURE VERIFICATION FAILED for ${installerPath} (${platform})`);
  process.exit(1);
}
console.log(`Signature verified: ${installerPath} (${platform})`);
