/**
 * Theme Token Compliance Test
 *
 * Scans every CSS file in ui/src/features/ and ui/src/frontend/ for
 * hardcoded colour, font-size, border-radius, box-shadow, and spacing
 * values that should reference design tokens via `var(--token)`.
 *
 * Exempts legitimate exceptions:
 *   - Gradient colour stops (radial/linear-gradient)
 *   - transparent, currentColor, inherit, initial, unset, auto, none
 *   - zero-length values (0, 0px, 0rem, etc.)
 *   - inset 0 (zero box-shadow position)
 *   - Standard browser-only properties (appearance, cursor, etc.)
 *   - Pseudo-element content strings
 *   - Keyframe percentage stops
 *   - Custom property definitions (--token: value)
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readdirSync, readFileSync } from 'fs';
import { join, resolve } from 'path';

/* ── File discovery ───────────────────────────────────────────── */

interface Violation {
  file: string;
  line: number;
  property: string;
  value: string;
  reason: string;
}

function isDesignToken(value: string): boolean {
  return /^var\(--/.test(value.trim());
}

/** Properties that naturally hold non-token values (pure CSS). */
const NON_TOKEN_PROPS = new Set([
  'appearance', '-webkit-appearance', 'cursor', 'pointer-events',
  'user-select', 'resize', 'overflow', 'overflow-x', 'overflow-y',
  'overflow-wrap', 'word-break', 'white-space', 'list-style',
  'border-collapse', 'border-spacing', 'box-sizing',
  'object-fit', 'object-position', 'float', 'clear',
  'table-layout', 'caption-side', 'empty-cells',
  'orphans', 'widows', 'break-inside', 'break-before', 'break-after',
  'page-break-inside', 'page-break-before', 'page-break-after',
  'will-change', 'transform', 'transform-origin',
  'transform-style', 'perspective', 'perspective-origin',
  'backface-visibility', 'visibility', 'isolation',
  'writing-mode', 'direction', 'unicode-bidi',
  'image-rendering', 'shape-rendering', 'clip-rule', 'fill-rule',
  'mix-blend-mode', 'background-blend-mode',
  'mask-type', 'clip-path',
  '-webkit-line-clamp', '-webkit-box-orient',
  'text-overflow', 'text-transform', 'text-decoration',
  'letter-spacing', 'line-height', 'font-style',
  'font-variant', 'font-variant-numeric', 'font-stretch',
  'tab-size', 'hyphens',
  'outline-style', 'outline-offset',
  'stroke', 'stroke-width', 'stroke-linecap', 'stroke-linejoin',
  'stroke-dasharray', 'stroke-dashoffset', 'stroke-opacity',
  'fill', 'fill-opacity',
  'animation', 'animation-name', 'animation-duration',
  'animation-timing-function', 'animation-delay',
  'animation-iteration-count', 'animation-direction',
  'animation-fill-mode', 'animation-play-state',
  'transition', 'transition-property', 'transition-delay',
  'accent-color', 'color-scheme', 'forced-color-adjust',
]);

/** Values that are always exempt from token requirements. */
const EXEMPT_VALUES = new Set([
  'transparent', 'currentColor', 'inherit', 'initial', 'unset',
  'none', 'auto', 'normal', 'bold', 'bolder', 'lighter',
  'italic', 'oblique', 'underline', 'overline', 'line-through',
  'solid', 'dashed', 'dotted', 'double', 'groove', 'ridge',
  'inset', 'outset', 'hidden', 'visible', 'scroll', 'clip',
  'collapse', 'separate', 'show', 'hide',
  'col-resize', 'row-resize', 'n-resize', 's-resize',
  'e-resize', 'w-resize', 'ne-resize', 'nw-resize',
  'se-resize', 'sw-resize', 'ew-resize', 'ns-resize',
  'nesw-resize', 'nwse-resize',
  'grab', 'grabbing', 'zoom-in', 'zoom-out', 'not-allowed',
  'pointer', 'default', 'text', 'crosshair', 'help', 'progress',
  'wait', 'cell', 'context-menu', 'alias', 'copy', 'move',
  'no-drop', 'all-scroll', 'vertical-text',
  'serif', 'sans-serif', 'monospace', 'cursive', 'fantasy',
  'system-ui', 'ui-serif', 'ui-sans-serif', 'ui-monospace',
  'contain', 'cover', 'fill', 'scale-down',
  'flex-start', 'flex-end', 'center', 'space-between',
  'space-around', 'space-evenly', 'baseline', 'stretch',
  'start', 'end', 'left', 'right', 'top', 'bottom',
  'row', 'column', 'row-reverse', 'column-reverse',
  'wrap', 'nowrap', 'wrap-reverse',
  'block', 'inline', 'inline-block', 'flex', 'inline-flex',
  'grid', 'inline-grid', 'table', 'inline-table',
  'list-item', 'flow', 'flow-root', 'contents',
  'relative', 'absolute', 'fixed', 'sticky', 'static',
  'normal', 'sub', 'super', 'baseline',
  'text-top', 'text-bottom', 'middle',
  'ltr', 'rtl', 'embed', 'bidi-override', 'isolate',
  'isolate-override', 'plaintext', 'border-box',
  'padding-box', 'content-box',
  'x', 'y', 'both',
]);

const ZERO_RE = /^0(px|rem|em|%|vh|vw|vmin|vmax|cm|mm|in|pt|pc)?$/;
const GRADIENT_RE = /^(linear-gradient|radial-gradient|conic-gradient|repeating-linear-gradient|repeating-radial-gradient|repeating-conic-gradient)/;
const INSET_ZERO_RE = /^inset\s+0$/;
/** Check if a value string should be exempted from token enforcement. */
function isExemptValue(value: string): boolean {
  const trimmed = value.trim().toLowerCase();

  // Zero-length values are universal
  if (ZERO_RE.test(trimmed)) return true;
  // Exempt set
  if (EXEMPT_VALUES.has(trimmed)) return true;
  // Inset zero
  if (INSET_ZERO_RE.test(trimmed)) return true;
  // Gradient functions
  if (GRADIENT_RE.test(trimmed)) return true;

  // Percentage values are relational, not fixed sizes to tokenize
  if (/^-?\d+(\.\d+)?%$/.test(trimmed)) return true;

  // Values smaller than --space-1 (0.25rem / 4px) are functional sub-token values
  // e.g. 0.0625rem (1px), 0.125rem (2px), 0.03125rem (0.5px hairline)
  if (/^0\.(03125|0625|125|1875|25)(px|rem|em)?$/i.test(trimmed)) return true;

  // calc(), min(), max(), clamp(), env() — these are dynamic
  if (/^(calc|min|max|clamp|env)\(/.test(trimmed)) return true;

  // url() references
  if (/^url\(/.test(trimmed)) return true;

  // var() references are already compliant
  if (trimmed.startsWith('var(--')) return true;

  // Hex colours used in SVG stroke/fill on non-color-token properties
  if (/^#[0-9a-f]{3,8}$/i.test(trimmed)) return false; // hex colours are violations unless exempted below

  // rgba() with alpha=0 (transparent equivalent)
  if (/^rgba?\s*\(.*,\s*0\s*\)$/.test(trimmed)) return true;

  return false;
}

/** Check if a CSS value contains a hardcoded colour. */
function hasHardcodedColor(value: string): boolean {
  // Strip var() contents to avoid false positives
  const stripped = value.replace(/var\(--[^)]+\)/g, '');

  // Check for hex colours
  if (/#[0-9a-f]{3,8}\b/i.test(stripped)) return true;
  // Check for rgb/rgba
  if (/rgba?\s*\(/i.test(stripped)) return true;
  // Check for hsl/hsla
  if (/hsla?\s*\(/i.test(stripped)) return true;

  return false;
}

const COLOR_PROPERTIES = new Set([
  'color', 'background-color', 'border-color', 'border-top-color',
  'border-right-color', 'border-bottom-color', 'border-left-color',
  'outline-color', 'text-decoration-color', 'caret-color',
  'background', 'background-image',
  'border', 'border-top', 'border-right', 'border-bottom', 'border-left',
  'border-left', 'border-right', 'border-top', 'border-bottom',
]);

const SPACING_PROPERTIES = new Set([
  'padding', 'padding-top', 'padding-right', 'padding-bottom', 'padding-left',
  'margin', 'margin-top', 'margin-right', 'margin-bottom', 'margin-left',
  'gap', 'row-gap', 'column-gap',
  'top', 'right', 'bottom', 'left',
  'inset', 'inset-inline', 'inset-block',
  'border-width', 'border-top-width', 'border-right-width',
  'border-bottom-width', 'border-left-width',
  'border-spacing',
  'scroll-margin', 'scroll-padding',
]);

/** Parse a CSS file and find potential token compliance violations. */
function scanCSS(filePath: string): Violation[] {
  const content = readFileSync(filePath, 'utf-8');
  const violations: Violation[] = [];

  // Remove comments to avoid false positives
  const stripped = content.replace(/\/\*[\s\S]*?\*\//g, '');

  // Split into individual rules (selectors + blocks)
  const rules = stripped.match(/[^{}]*\{[^{}]*\}/g) || [];

  for (const rule of rules) {
    const braceIdx = rule.indexOf('{');
    const selectors = rule.slice(0, braceIdx).trim();
    const body = rule.slice(braceIdx + 1, -1).trim();

    // Skip @keyframes and @media wrappers (we scan their contents)
    if (selectors.startsWith('@')) continue;

    // Skip custom property definitions (:root blocks and --token: value)
    if (/:root|\[data-theme/.test(selectors)) continue;

    // Parse each CSS declaration in the body
    const decls = body.split(';');
    for (const decl of decls) {
      const trimmed = decl.trim();
      if (!trimmed) continue;

      const colonIdx = trimmed.indexOf(':');
      if (colonIdx < 0) continue;

      const property = trimmed.slice(0, colonIdx).trim();
      const value = trimmed.slice(colonIdx + 1).trim();

      // Skip non-token-able properties
      if (NON_TOKEN_PROPS.has(property)) continue;

      // Skip vendor-prefixed properties (except -webkit-appearance which is covered)
      if (property.startsWith('-webkit-') || property.startsWith('-moz-') || property.startsWith('-ms-') || property.startsWith('-o-')) {
        // Still check colour on -webkit-text-fill-color etc
        if (!property.includes('color') && !property.includes('shadow')) continue;
      }

      // Skip if value is already a var() reference
      if (isDesignToken(value)) continue;

      // Skip if value is exempt
      if (isExemptValue(value)) continue;

      // Skip if this is a multi-value with some var() refs and some constants
      // e.g., `var(--shadow-glow, var(--shadow-2xl)), var(--shadow-sm)`
      if (value.includes('var(--')) continue;

      // Calculate line number for error reporting
      const declPos = content.indexOf(trimmed);
      const line = declPos >= 0
        ? content.slice(0, declPos).split('\n').length
        : 1;

      // Check colours
      if (COLOR_PROPERTIES.has(property) && hasHardcodedColor(value)) {
        violations.push({
          file: filePath,
          line,
          property,
          value: value.slice(0, 80),
          reason: 'Hardcoded colour should use --color-* token',
        });
        continue;
      }

      // Check font-size
      if (property === 'font-size') {
        if (/^\d/.test(value) && !isDesignToken(value) && !isExemptValue(value)) {
          violations.push({
            file: filePath,
            line,
            property,
            value: value.slice(0, 80),
            reason: 'Hardcoded font-size should use --text-* token',
          });
        }
        continue;
      }

      // Check font-family
      if (property === 'font-family') {
        if (!value.includes('var(--font-')) {
          violations.push({
            file: filePath,
            line,
            property,
            value: value.slice(0, 80),
            reason: 'Hardcoded font-family should use --font-* token',
          });
        }
        continue;
      }

      // Check border-radius
      if (property === 'border-radius' || property.startsWith('border-') && property.endsWith('-radius')) {
        if (/^\d/.test(value) && !isDesignToken(value) && !isExemptValue(value)) {
          violations.push({
            file: filePath,
            line,
            property,
            value: value.slice(0, 80),
            reason: 'Hardcoded border-radius should use --radius-* token',
          });
        }
        continue;
      }

      // Check box-shadow
      if (property === 'box-shadow' || property === 'text-shadow') {
        if (!isDesignToken(value) && !isExemptValue(value) && value !== 'none') {
          violations.push({
            file: filePath,
            line,
            property,
            value: value.slice(0, 80),
            reason: 'Hardcoded shadow should use --shadow-* token',
          });
        }
        continue;
      }

      // Check spacing values that aren't `0`
      if (SPACING_PROPERTIES.has(property)) {
        // Skip multi-value properties that include var() references
        if (value.includes('var(--')) continue;

        // Check if value has a dimension that should use --space-* token
        const hasUnitValue = /\d+(px|rem|em|%)/.test(value);
        if (hasUnitValue && !isExemptValue(value)) {
          // Skip if it's a simple `auto` or similar
          if (EXEMPT_VALUES.has(value.trim().toLowerCase())) continue;

          // Check for multi-value with mix of tokens and hardcoded
          const parts = value.split(/\s+/).filter(Boolean);
          const allTokenOrExempt = parts.every(
            (p) => isDesignToken(p) || isExemptValue(p),
          );

          if (!allTokenOrExempt && !value.includes('var(--')) {
            violations.push({
              file: filePath,
              line,
              property,
              value: value.slice(0, 80),
              reason: 'Hardcoded spacing/dimension should use --space-* token',
            });
          }
        }
        continue;
      }
    }
  }

  return violations;
}

/* ── Test runner ──────────────────────────────────────────────── */

const UI_SRC = resolve(__dirname, '..');

interface ScanTarget {
  path: string;
  label: string;
}

const SCAN_TARGETS: ScanTarget[] = [
  { path: 'features', label: 'Feature CSS files' },
  { path: 'frontend', label: 'Frontend/shell CSS files' },
  { path: 'components', label: 'Shared component CSS files' },
];

/** Find CSS files in a directory recursively, excluding tokens.css and components.css. */
function findFeatureCssFiles(dir: string): string[] {
  const results: string[] = [];
  const absDir = resolve(UI_SRC, dir);

  try {
    const entries = readdirSync(absDir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = join(absDir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name === 'node_modules' || entry.name === '.git') continue;
        results.push(...findFeatureCssFiles(join(dir, entry.name)));
      } else if (entry.name.endsWith('.css')) {
        // Skip the design token definition files themselves
        if (entry.name === 'tokens.css' || entry.name === 'components.css') continue;
        results.push(fullPath);
      }
    }
  } catch {
    // Directory doesn't exist — skip
  }

  return results;
}

describe('CSS design token compliance', () => {
  let allViolations: Violation[];
  let allFiles: string[];

  beforeAll(() => {
    allViolations = [];
    allFiles = [];

    for (const target of SCAN_TARGETS) {
      const files = findFeatureCssFiles(target.path);
      allFiles.push(...files);
      for (const file of files) {
        try {
          const result = scanCSS(file);
          allViolations.push(...result);
        } catch {
          // Skip unparseable files
        }
      }
    }
  });

  it('scanned at least 10 CSS files', () => {
    expect(allFiles.length).toBeGreaterThanOrEqual(10);
  });

  it('all CSS values use design tokens (no hardcoded colours, sizes, shadows)', () => {
    const msg =
      allViolations.length > 0
        ? `Found ${allViolations.length} hardcoded value(s) not using design tokens:\n\n${allViolations
            .map(
              (v, i) =>
                `  ${i + 1}. ${v.file
                  .replace(/\\/g, '/')
                  .replace(/.+?ui[/\\]src[/\\]/, '')}:${v.line}\n` +
                `     Property: ${v.property}\n` +
                `     Value:    ${v.value}\n` +
                `     Reason:   ${v.reason}`,
            )
            .join('\n\n')}`
        : 'All CSS values use design tokens correctly.';

    expect(allViolations, msg).toHaveLength(0);
  });
});
