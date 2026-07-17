#!/usr/bin/env node
// ── OZ-POS Updater Manifest Generator ─────────────────────────────────
//
// Generates a Tauri v2 updater `latest.json` manifest with Ed25519
// signatures over the installer binaries. Used by the release workflow
// to attach a signed manifest to every GitHub Release.
//
// Usage:
//   node scripts/generate-latest-json.mjs <version> <notes> <platform> <installer-path>
//
// Example:
//   node scripts/generate-latest-json.mjs 0.1.0 "Bug fixes" windows-x86_64 ./bundle/nsis/OZ-POS_0.1.0_x64-setup.exe
//
// Environment:
//   UPDATER_PRIVATE_KEY — Ed25519 private key (64 hex chars or base64)
//
// Output:
//   A valid latest.json platform fragment to stdout.
//
// Requirements:
//   Node.js 22+ (uses native Ed25519 via crypto.subtle)

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

// subtle is under webcrypto in Node.js
const { subtle } = globalThis.crypto;

const [_node, _script, version, notes, platform, installerPath] = process.argv;

if (!version || !notes || !platform || !installerPath) {
  console.error(
    "Usage: node generate-latest-json.mjs <version> <notes> <platform> <installer-path>"
  );
  process.exit(1);
}

const privateKeyEnv = process.env.UPDATER_PRIVATE_KEY;
if (!privateKeyEnv) {
  console.error("UPDATER_PRIVATE_KEY environment variable is not set");
  process.exit(1);
}

// Parse the private key. It may be raw hex (64 chars → 32 bytes) or
// base64-encoded raw bytes (matching what `openssl genpkey` produces).
let privateKeyBytes;
if (/^[0-9a-fA-F]{64}$/.test(privateKeyEnv)) {
  privateKeyBytes = Buffer.from(privateKeyEnv, "hex");
} else {
  privateKeyBytes = Buffer.from(privateKeyEnv, "base64");
}

if (privateKeyBytes.length !== 32) {
  console.error(
    `Ed25519 private key must be 32 bytes, got ${privateKeyBytes.length}`
  );
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

const installerHash = createHash("sha256")
  .update(installerBytes)
  .digest("hex");
console.error(`Installer SHA-256: ${installerHash}`);

// Sign the raw installer bytes with Ed25519.
const key = await subtle.importKey(
  "raw",
  privateKeyBytes,
  { name: "Ed25519" },
  false,
  ["sign"]
);

const signature = await subtle.sign({ name: "Ed25519" }, key, installerBytes);
const signatureBase64 = Buffer.from(signature).toString("base64");

// The release URL is deterministic based on the tag name.
const repo = process.env.REPO || "kardelitaitu/oz-pos";
const filename = installerPath.split("/").pop().split("\\").pop();
const url = `https://github.com/${repo}/releases/download/v${version}/${filename}`;

const manifest = {
  version,
  notes,
  pub_date: new Date().toISOString(),
  platforms: {
    [platform]: {
      signature: signatureBase64,
      url,
    },
  },
};

process.stdout.write(JSON.stringify(manifest, null, 2) + "\n");
