#!/usr/bin/env node
// ── OZ-POS Updater Manifest Generator ─────────────────────────────────
//
// Generates a Tauri v2 updater `latest.json` manifest with Ed25519
// signatures over the installer binaries. Used by the release workflow
// to attach a signed manifest to every GitHub Release.
//
// Usage:
//   node scripts/generate-latest-json.mjs <version> <notes> <platform> <installer-path> [--min-version <version>]
//   node scripts/generate-latest-json.mjs <version> <notes> <platform> <installer-path> --merge <manifest.json> [--verify-pubkey <base64>] [--min-version <version>]
//   node scripts/generate-latest-json.mjs --self-test
//
// Examples:
//   node scripts/generate-latest-json.mjs 0.1.0 "Bug fixes" windows-x86_64 ./bundle/nsis/OZ-POS_0.1.0_x64-setup.exe --min-version 0.0.18
//   # Multi-platform: run once per platform, merging into one manifest:
//   node scripts/generate-latest-json.mjs 0.1.0 "Notes" linux-x86_64 ./x.AppImage --verify-pubkey "$PUBKEY" > latest.json
//   node scripts/generate-latest-json.mjs 0.1.0 "Notes" windows-x86_64 ./x-setup.exe --merge latest.json --verify-pubkey "$PUBKEY" > latest.json.tmp && mv latest.json.tmp latest.json
//
// Environment:
//   UPDATER_PRIVATE_KEY — Ed25519 private key seed (64 hex chars or base64 of 32 bytes)
//   REPO               — GitHub repo for deterministic release URLs (default kardelitaitu/oz-pos)
//
// Output:
//   A valid latest.json platform manifest (single platform, or merged when --merge is used).
//
// Requirements:
//   Node.js 20+ (node:crypto Ed25519 via PKCS8/SPKI DER)
//
// AUDIT-28 RELEASE-04: `--verify-pubkey` derives the public key from the
// private seed and fails if it does not match the Tauri-format public key
// (raw 32 bytes, or the "untrusted comment: minisign public key:" base64
// blob stored in tauri.conf.json::plugins.updater.pubkey). This closes the
// key-encoding compatibility gap: a key that cannot verify against the
// committed pubkey is rejected before any release artifact is published.
// `--self-test` runs a sign → derive → verify round-trip so CI can prove
// the toolchain works without real key material.
//
// NOTE: WebCrypto (crypto.subtle) raw Ed25519 import rejects the `sign`
// usage on Node 24, so this script uses the low-level node:crypto API with
// explicit PKCS8/SPKI DER key encoding — deterministic across Node versions.

import { createHash, createPrivateKey, createPublicKey, sign, verify } from "node:crypto";
import { readFileSync } from "node:fs";

// DER prefixes (RFC 8410):
//   PKCS8 private: SEQUENCE(30 2e) { INTEGER 0, AlgId Ed25519, OCTET STRING { OCTET STRING seed } }
//   SPKI public:   SEQUENCE(30 2a) { AlgId Ed25519, BIT STRING pubkey }
const PKCS8_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");
const SPKI_PREFIX = Buffer.from("302a300506032b6570032100", "hex");

// ── Argument parsing ───────────────────────────────────────────────
function parseArgs(argv) {
  let minVersion;
  let mergePath;
  let verifyPubkey;
  const positional = [];
  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--min-version") {
      minVersion = argv[++i];
    } else if (arg === "--merge") {
      mergePath = argv[++i];
    } else if (arg === "--verify-pubkey") {
      verifyPubkey = argv[++i];
    } else if (arg === "--self-test") {
      positional.push("--self-test");
    } else {
      positional.push(arg);
    }
  }
  return { minVersion, mergePath, verifyPubkey, positional };
}

// ── Key helpers ────────────────────────────────────────────────────
// Accepts 64-hex (32 bytes) or base64 of the raw 32-byte seed.
function parsePrivateKey(env) {
  let bytes;
  if (/^[0-9a-fA-F]{64}$/.test(env)) {
    bytes = Buffer.from(env, "hex");
  } else {
    bytes = Buffer.from(env, "base64");
  }
  if (bytes.length !== 32) {
    throw new Error(
      `Ed25519 private key must be 32 bytes, got ${bytes.length} (secret must be the raw 32-byte seed, hex or base64)`
    );
  }
  return bytes;
}

function privateKeyFromSeed(seed) {
  return createPrivateKey({
    key: Buffer.concat([PKCS8_PREFIX, seed]),
    format: "der",
    type: "pkcs8",
  });
}

function publicKeyFromRaw(pubkeyRaw) {
  return createPublicKey({
    key: Buffer.concat([SPKI_PREFIX, pubkeyRaw]),
    format: "der",
    type: "spki",
  });
}

// Decodes a public key that may be:
//   - raw base64 of the 32-byte Ed25519 key
//   - a minisign-style base64 blob (tauri.conf.json updater.pubkey),
//     whose final line base64-decodes to 0x45-prefixed 33-byte key
function parsePublicKey(pubkeyB64) {
  const buf = Buffer.from(pubkeyB64, "base64");
  const text = buf.toString("utf8");
  if (text.includes("untrusted comment") || text.includes("\n")) {
    const lines = text.split("\n").map((l) => l.trim()).filter(Boolean);
    const keyLine = lines[lines.length - 1];
    return stripMinisignPrefix(Buffer.from(keyLine, "base64"));
  }
  return stripMinisignPrefix(buf);
}

function stripMinisignPrefix(bytes) {
  // minisign Ed25519 keys carry a 0x45 ('E') tag byte before the 32-byte key
  if (bytes.length === 33 && bytes[0] === 0x45) return bytes.subarray(1);
  return bytes;
}

function derivePublicKey(seed) {
  const jwk = privateKeyFromSeed(seed).export({ format: "jwk" });
  return Buffer.from(jwk.x, "base64url");
}

function signBytes(seed, bytes) {
  return sign(null, bytes, privateKeyFromSeed(seed));
}

function verifyBytes(pubkeyRaw, bytes, signature) {
  return verify(null, bytes, publicKeyFromRaw(pubkeyRaw), signature);
}

function toMinisignPubkey(raw) {
  // Format matches tauri signer generate output used in tauri.conf.json
  const tagged = Buffer.concat([Buffer.from([0x45]), raw]);
  const id = Buffer.from(raw.subarray(0, 8)).toString("hex").toUpperCase();
  const body = `untrusted comment: minisign public key: ${id}\n${tagged
    .toString("base64")
    .replace(/(.{64})/g, "$1\n")}`;
  return Buffer.from(body, "utf8").toString("base64");
}

// ── Manifest assembly ──────────────────────────────────────────────
function buildFragment({ version, notes, platform, signature, url, minVersion }) {
  const fragment = {
    version,
    notes,
    pub_date: new Date().toISOString(),
    platforms: {
      [platform]: { signature, url },
    },
  };
  if (minVersion) fragment.min_version = minVersion;
  return fragment;
}

function mergeManifest(existing, fragment) {
  return {
    ...existing,
    ...fragment,
    platforms: {
      ...(existing.platforms || {}),
      ...fragment.platforms,
    },
  };
}

// ── Self-test: proves sign → derive → verify round-trip ───────────
function selfTest() {
  const seed = Buffer.alloc(32, 0x42); // deterministic test seed
  const payload = Buffer.from("oz-pos updater self-test payload", "utf8");

  const pubkeyRaw = derivePublicKey(seed);
  if (pubkeyRaw.length !== 32) {
    console.error(`FAIL pubkey derivation: expected 32 bytes, got ${pubkeyRaw.length}`);
    process.exit(1);
  }

  const signature = signBytes(seed, payload);
  if (signature.length !== 64) {
    console.error(`FAIL signature length: expected 64 bytes, got ${signature.length}`);
    process.exit(1);
  }
  const ok = verifyBytes(pubkeyRaw, payload, signature);
  if (!ok) {
    console.error("FAIL sign/verify round-trip");
    process.exit(1);
  }

  // minisign-format pubkey must round-trip through parsePublicKey
  const minisign = toMinisignPubkey(pubkeyRaw);
  const reParsed = parsePublicKey(minisign);
  if (!reParsed.equals(pubkeyRaw)) {
    console.error("FAIL minisign pubkey re-parse");
    process.exit(1);
  }

  // tampered payload must be rejected
  const tampered = verifyBytes(pubkeyRaw, Buffer.concat([payload, Buffer.from([0])]), signature);
  if (tampered) {
    console.error("FAIL tampered payload accepted");
    process.exit(1);
  }

  console.log("self-test: Ed25519 sign/derive/verify + minisign pubkey round-trip PASSED");
  process.exit(0);
}

// ── Main ───────────────────────────────────────────────────────────
const { minVersion, mergePath, verifyPubkey, positional } = parseArgs(process.argv);

if (positional[0] === "--self-test") {
  selfTest();
}

const [version, notes, platform, installerPath] = positional;

if (!version || !notes || !platform || !installerPath) {
  console.error(
    "Usage: node generate-latest-json.mjs <version> <notes> <platform> <installer-path> [--min-version <version>] [--merge <manifest.json>] [--verify-pubkey <base64>]"
  );
  process.exit(1);
}

const privateKeyEnv = process.env.UPDATER_PRIVATE_KEY;
if (!privateKeyEnv) {
  console.error("UPDATER_PRIVATE_KEY environment variable is not set");
  process.exit(1);
}

let seed;
try {
  seed = parsePrivateKey(privateKeyEnv);
} catch (e) {
  console.error(e.message);
  process.exit(1);
}

// Read the installer binary. The Ed25519 signature is computed over
// the raw bytes (matching `tauri signer sign`), not over a hash.
let installerBytes;
try {
  installerBytes = readFileSync(installerPath);
} catch (e) {
  console.error(`Failed to read installer: ${e.message}`);
  process.exit(1);
}

const installerHash = createHash("sha256").update(installerBytes).digest("hex");
console.error(`Installer SHA-256: ${installerHash}`);

// RELEASE-04: fail fast if the seed does not match the committed pubkey.
if (verifyPubkey) {
  let expected;
  try {
    expected = parsePublicKey(verifyPubkey);
  } catch {
    console.error("Failed to decode --verify-pubkey value (expected base64)");
    process.exit(1);
  }
  const derived = derivePublicKey(seed);
  if (!derived.equals(expected)) {
    console.error(
      `PUBKEY MISMATCH: private seed derives to ${derived.toString("hex")} but ` +
        `--verify-pubkey decodes to ${expected.toString("hex")}. The signing secret ` +
        `does not match the pubkey embedded in tauri.conf.json — refusing to sign.`
    );
    process.exit(1);
  }
  console.error("Pubkey check PASSED: seed matches the committed updater pubkey");
}

const signature = signBytes(seed, installerBytes).toString("base64");

// The release URL is deterministic based on the tag name.
const repo = process.env.REPO || "kardelitaitu/oz-pos";
const filename = installerPath.split("/").pop().split("\\").pop();
const url = `https://github.com/${repo}/releases/download/v${version}/${filename}`;

const fragment = buildFragment({ version, notes, platform, signature, url, minVersion });

let manifest;
if (mergePath) {
  let existing;
  try {
    existing = JSON.parse(readFileSync(mergePath, "utf8"));
  } catch (e) {
    console.error(`Failed to read --merge manifest ${mergePath}: ${e.message}`);
    process.exit(1);
  }
  manifest = mergeManifest(existing, fragment);
} else {
  manifest = fragment;
}

process.stdout.write(JSON.stringify(manifest, null, 2) + "\n");
