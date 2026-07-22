/**
 * scripts/fix-tbody.js — Batch-fix whitespace text nodes inside <tbody>.
 *
 * Fixes two patterns:
 *   1. Opening:  <tbody>\n<whitespace>{  → <tbody>{
 *   2. Closing:  <whitespace></tbody>    → </tbody>
 *
 * Run from workspace root:  node scripts/fix-tbody.js
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// Find all .tsx files with <tbody> at end of line (the problematic pattern)
const result = execSync(
  'grep -rln "<tbody>$" --include="*.tsx" --include="*.ts" src/',
  { cwd: path.join(__dirname, '..', 'ui'), encoding: 'utf8' }
);

const files = result.trim().split('\n').filter(Boolean);
console.log(`Found ${files.length} files with <tbody> at end of line.`);

let totalFixes = 0;
let totalOpenFixes = 0;
let totalCloseFixes = 0;

for (const filePath of files) {
  const fullPath = path.join(__dirname, '..', 'ui', filePath);
  let content = fs.readFileSync(fullPath, 'utf8');
  let modified = false;

  // Fix 1: Opening pattern — <tbody>\r?\n<whitespace>{ → <tbody>{
  // Handles both Unix \n and Windows \r\n line endings
  const openRegex = /<tbody>\r?\n[ \t]*\{/g;
  const openMatches = content.match(openRegex);
  if (openMatches) {
    totalOpenFixes += openMatches.length;
    content = content.replace(openRegex, '<tbody>{');
    modified = true;
  }

  // Fix 2: Closing pattern — <whitespace></tbody> → </tbody>
  // Remove whitespace before </tbody> at end of line
  const closeRegex = /[ \t]+<\/tbody>/g;
  const closeMatches = content.match(closeRegex);
  if (closeMatches) {
    totalCloseFixes += closeMatches.length;
    content = content.replace(closeRegex, '</tbody>');
    modified = true;
  }

  if (modified) {
    fs.writeFileSync(fullPath, content, 'utf8');
    totalFixes++;
    console.log(`  Fixed: ${filePath}`);
  }
}

console.log(`\nDone! Fixed ${totalFixes} files:`);
console.log(`  Opening fixes: ${totalOpenFixes}`);
console.log(`  Closing fixes: ${totalCloseFixes}`);
