/**
 * Popup Background Compliance
 *
 * Every popup, dropdown, tooltip, modal, popover, menu, dialog, toast,
 * and floating surface container MUST have a visible background. A missing
 * or transparent background makes text unreadable against page content.
 *
 * This test auto-discovers surfaces by CSS class naming patterns and
 * verifies each one has a `background` or `background-color` declaration
 * that is NOT `transparent`, `none`, or `inherit`.
 *
 * Rules:
 *   - Only checks the ROOT container class (not children like -head, -body, -icon)
 *   - Excludes overlays (purposefully translucent dimmers)
 *   - Excludes pseudo-elements, state modifiers (:hover, :focus, --exiting)
 *   - Excludes buttons/inputs inside popups (they have their own styling)
 */

import { describe, it, expect } from 'vitest';
import { readFileSync, readdirSync, statSync } from 'fs';
import { resolve, relative, join } from 'path';

const UI_SRC = resolve(__dirname, '..');

/* ── Root container patterns (these need backgrounds) ───────────── */
const POPUP_ROOT = /(?:^|[\s,.])(?:popup|dropdown|tooltip|modal|popover|menu|dialog|toast|picker)(?:$|[\s{,:])/i;

/* ── Exclude: overlays, child elements, state modifiers ─────────── */
const IS_OVERLAY = /overlay/i;
const IS_CHILD = /(?:head|header|body|content|footer|close|icon|title|label|badge|arrow|pointer|item|entry|row|meta|summary|error|backdrop)(?:$|-)/i;
const IS_STATE = /(?:hover|focus|active|disabled|open|closed|exiting|entering|visible|hidden|selected|checked)/i;
const IS_PSEUDO = /::/;

/* ── Background values that indicate missing/transparent bg ─────── */
const NO_BG = /^(transparent|none|inherit|initial|unset)$/i;
const TRANSLUCENT_TOKEN = /var\(--color-bg-overlay\)/;

/* ── Collect all CSS files ──────────────────────────────────────── */
function collectCssFiles(dir: string): string[] {
  const files: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      if (entry === 'node_modules' || entry === '__tests__') continue;
      files.push(...collectCssFiles(full));
    } else if (entry.endsWith('.css') && !entry.endsWith('.test.css')) {
      files.push(full);
    }
  }
  return files;
}

/* ── Extract CSS rules (selector + body + line) ─────────────────── */
interface CssRule {
  selector: string;
  body: string;
  line: number;
}

function extractRules(css: string): CssRule[] {
  const clean = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const rules: CssRule[] = [];
  let i = 0;
  while (i < clean.length) {
    const braceStart = clean.indexOf('{', i);
    if (braceStart === -1) break;
    const selector = clean.slice(i, braceStart).trim();
    // Skip @media, @keyframes, @supports, etc. — only check top-level rules
    if (selector.startsWith('@')) {
      // Skip past the entire @-block (including nested rules)
      let depth = 1;
      let pos = braceStart + 1;
      while (pos < clean.length && depth > 0) {
        if (clean[pos] === '{') depth++;
        else if (clean[pos] === '}') depth--;
        pos++;
      }
      i = pos;
      continue;
    }
    let depth = 1;
    let pos = braceStart + 1;
    while (pos < clean.length && depth > 0) {
      if (clean[pos] === '{') depth++;
      else if (clean[pos] === '}') depth--;
      pos++;
    }
    if (depth === 0) {
      const body = clean.slice(braceStart + 1, pos - 1);
      const line = clean.slice(0, braceStart).split('\n').length;
      rules.push({ selector, body, line });
    }
    i = pos;
  }
  return rules;
}

/* ── Pull background declaration ────────────────────────────────── */
function getBackground(body: string): string | null {
  const bgColor = body.match(/(?:^|;)\s*background-color\s*:\s*([^;]+)/);
  if (bgColor) return bgColor[1]!.trim();
  const bg = body.match(/(?:^|;)\s*background\s*:\s*([^;]+)/);
  if (bg) return bg[1]!.trim();
  return null;
}

/* ── Extract the primary class name from a selector ─────────────── */
function primaryClass(selector: string): string | null {
  const m = selector.match(/\.([a-zA-Z][\w-]*)/);
  return m ? m[1]! : null;
}

/* ── Tests ──────────────────────────────────────────────────────── */

describe('popup surfaces have visible backgrounds', () => {
  const cssFiles = collectCssFiles(UI_SRC);
  const failures: string[] = [];

  for (const filePath of cssFiles) {
    const content = readFileSync(filePath, 'utf-8');
    const rules = extractRules(content);
    const relPath = relative(UI_SRC, filePath);

    for (const rule of rules) {
      // Skip pseudo-elements
      if (IS_PSEUDO.test(rule.selector)) continue;

      // Must contain a popup-like class
      if (!POPUP_ROOT.test(rule.selector)) continue;

      // Get the primary class — skip child/state/overlay classes
      const cls = primaryClass(rule.selector);
      if (!cls) continue;
      if (IS_OVERLAY.test(cls)) continue;
      if (IS_CHILD.test(cls)) continue;
      if (IS_STATE.test(cls)) continue;

      const bg = getBackground(rule.body);

      // No background at all
      if (bg === null) {
        failures.push(
          `${relPath}:${rule.line} — ${rule.selector}\n` +
          `  No background/background-color declaration`,
        );
        continue;
      }

      // Transparent / none / inherit
      if (NO_BG.test(bg)) {
        failures.push(
          `${relPath}:${rule.line} — ${rule.selector}\n` +
          `  Background is "${bg}" (must be visible)`,
        );
        continue;
      }

      // Translucent overlay token used on a popup container
      if (TRANSLUCENT_TOKEN.test(bg)) {
        failures.push(
          `${relPath}:${rule.line} — ${rule.selector}\n` +
          `  Uses --color-bg-overlay (${bg}) — too translucent for readable text`,
        );
      }
    }
  }

  it(`every popup container has an opaque background (${cssFiles.length} CSS files scanned)`, () => {
    expect(failures, `\n${failures.join('\n\n')}`).toEqual([]);
  });
});
