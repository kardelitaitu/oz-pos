#!/usr/bin/env node
/**
 * Generate _scoped variants for tablet command modules (v3).
 *
 * Strategy: For each unscoped fn that uses state.db.lock().await:
 *   1. Replace `let conn = state.db.lock().await;` with
 *      `let (_session, conn) = state.resolve_scope(&session_token)?;\n    let db = conn.lock().map_err(...)?;\n    let conn = &*db;`
 *   2. Add session_token as first param
 *   3. Remove State param
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { basename } from 'path';

const TABLET_DIR = 'apps/tablet-client/src/commands';

const SKIP_COMMANDS = new Set([
  'staff_login', 'create_session', 'destroy_session', 'session_keepalive',
  'staff_check_username', 'bootstrap_owner', 'has_users',
  'ping', 'version', 'get_device_id', 'get_local_ip',
  'currency_info', 'pick_logo_file', 'open_product_images',
  'list_workspaces', 'list_workspace_screens', 'resolve_boot_store',
  'list_all_features', 'set_feature', 'set_features_bulk',
  'complete_setup', 'dismiss_setup_wizard', 'get_enabled_features', 'get_setup_status',
  'get_subscription_capabilities',
  'activate_license', 'renew_license', 'pause_subscription', 'resume_subscription',
  'get_license_status', 'get_machine_id', 'get_hardware_fingerprint',
  'check_license_status', 'test_auth_connection',
]);

const SKIP_MODULES = new Set([
  'auth', 'authz', 'health', 'license', 'setup', 'features', 'subscription',
  'analytics', 'audit', 'currencies', 'customers', 'exchange_rates',
  'inventory_counts', 'loyalty', 'pos', 'refunds', 'reports',
  'staff', 'stock_transfers', 'tax', 'void',
]);

function processFile(filePath) {
  if (!existsSync(filePath)) return { added: 0, error: 'file not found' };

  const src = readFileSync(filePath, 'utf8');
  const modName = basename(filePath, '.rs');
  if (SKIP_MODULES.has(modName)) return { added: 0, skipped: 'module skip-listed' };

  const testIdx = src.indexOf('#[cfg(test)]');
  const insertIdx = testIdx !== -1 ? testIdx : src.length;

  const fnPattern = /pub async fn ([a-z_]+)\(/g;
  let match;
  let additions = '';
  let added = 0;

  while ((match = fnPattern.exec(src)) !== null) {
    const fnName = match[1];
    if (fnName.endsWith('_scoped') || SKIP_COMMANDS.has(fnName)) continue;
    if (src.includes('pub async fn ' + fnName + '_scoped')) continue;

    const sigStart = match.index;
    const bodyStart = src.indexOf('{', sigStart);
    if (bodyStart === -1) continue;

    let depth = 0;
    let fnEnd = bodyStart;
    for (let i = bodyStart; i < src.length; i++) {
      if (src[i] === '{') depth++;
      if (src[i] === '}') { depth--; if (depth === 0) { fnEnd = i + 1; break; } }
    }

    const fullFn = src.substring(sigStart, fnEnd);
    const sig = fullFn.substring(0, fullFn.indexOf('{'));

    // Extract params using angle-bracket-aware parsing
    const sigOpenParen = sig.indexOf('(', sig.indexOf('fn ') + 3);
    let aDepth = 0;
    let sigParamEnd = -1;
    for (let i = sigOpenParen + 1; i < sig.length - 3; i++) {
      if (sig[i] === '<') aDepth++;
      if (sig[i] === '>') aDepth--;
      if (aDepth === 0 && sig.substring(i, i + 4) === ') ->') {
        sigParamEnd = i;
        break;
      }
    }
    if (sigOpenParen === -1 || sigParamEnd === -1) continue;
    const rawParams = sig.substring(sigOpenParen + 1, sigParamEnd);

    // Split params at top level (respecting angle brackets)
    const topParts = [];
    let current = '';
    let angleD = 0;
    for (const ch of rawParams) {
      if (ch === '<') angleD++;
      if (ch === '>') angleD--;
      if (ch === ',' && angleD === 0) {
        topParts.push(current.trim());
        current = '';
      } else {
        current += ch;
      }
    }
    if (current.trim()) topParts.push(current.trim());

    // Remove state param, add session_token
    const scopedParams = topParts
      .filter(p => !p.includes('State<') && p.length > 0);
    scopedParams.unshift('    session_token: String');
    const paramStr = scopedParams.join(',\n    ');

    // Extract body
    const body = fullFn.substring(fullFn.indexOf('{'));
    const innerBody = body.replace(/^\{\s*/, '').replace(/\s*\}$/, '');

    // Get return type
    const retMatch = sig.match(/-> Result<([\s\S]*?),\s*AppError>/);
    if (!retMatch) continue;
    const retType = retMatch[1];

    // Check patterns
    const usesDbLock = /let (?:conn|db) = state\.db\.lock\(\)\.await;/.test(innerBody);
    const hasUserPerm = innerBody.includes('require_permission_for_user') && innerBody.includes('args.user_id');

    let scopedBody;
    if (usesDbLock && hasUserPerm) {
      scopedBody = innerBody
        .replace(
          /let db = state\.db\.lock\(\)\.await;/,
          'let (session, conn_arc) = state.resolve_scope(&session_token)?;\n    let db_guard = conn_arc.lock().map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;\n    let db = &*db_guard;'
        )
        .replace(
          /let conn = state\.db\.lock\(\)\.await;/,
          'let (session, conn_arc) = state.resolve_scope(&session_token)?;\n    let db_guard = conn_arc.lock().map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;\n    let conn = &*db_guard;'
        )
        .replace(/args\.user_id/g, 'session.user_id');
    } else if (usesDbLock) {
      scopedBody = innerBody
        .replace(
          /let db = state\.db\.lock\(\)\.await;/,
          'let (_session, conn_arc) = state.resolve_scope(&session_token)?;\n    let db_guard = conn_arc.lock().map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;\n    let db = &*db_guard;'
        )
        .replace(
          /let conn = state\.db\.lock\(\)\.await;/,
          'let (_session, conn_arc) = state.resolve_scope(&session_token)?;\n    let db_guard = conn_arc.lock().map_err(|e| AppError::Internal(format!("store db lock: {e}")))?;\n    let conn = &*db_guard;'
        );
    } else {
      scopedBody = `    let _session = state.resolve_session(&session_token)?;\n` + innerBody;
    }

    const scopedName = fnName + '_scoped';
    additions += `\n/// Session-scoped variant of \`${fnName}\`.\n#[command]\npub async fn ${scopedName}(\n    ${paramStr},\n    state: State<'_, AppState>,\n) -> Result<${retType}, AppError> {\n${scopedBody}\n}\n`;
    added++;
  }

  if (added > 0) {
    const before = src.substring(0, insertIdx).trimEnd();
    const after = src.substring(insertIdx);
    writeFileSync(filePath, before + '\n' + additions + '\n' + after);
  }

  return { added };
}

// ── Main ──────────────────────────────────────────────────────────

const files = [
  'branding.rs', 'browser.rs', 'bundles.rs', 'categories.rs',
  'gift_cards.rs', 'hardware.rs', 'history.rs', 'kds.rs',
  'offline.rs', 'pos.rs', 'product_variants.rs', 'products.rs',
  'promotions.rs', 'purchasing.rs', 'scale.rs', 'settings.rs',
  'sync.rs', 'tables.rs', 'terminals.rs', 'workspaces.rs',
];

let totalAdded = 0;

for (const file of files) {
  const path = `${TABLET_DIR}/${file}`;
  const result = processFile(path);
  const name = basename(file, '.rs');
  if (result.added > 0) {
    console.log(`✅ ${name}: +${result.added} scoped variants`);
    totalAdded += result.added;
  } else {
    console.log(`⏭️  ${name}: ${result.skipped || result.error || 'no changes'}`);
  }
}

console.log(`\nTotal: +${totalAdded} scoped variants generated`);
