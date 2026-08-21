#!/usr/bin/env node
/**
 * validate-test-coverage-doc.ts
 *
 * Validates that TEST_COVERAGE_ANALYSIS.md is consistent with reality.
 * Run as part of CI or pre-commit to enforce the mandatory update rule.
 *
 * Checks:
 * 1. Components in "Without Tests" tables actually have no test file
 * 2. Components in "Recently Covered" actually have test files
 * 3. Coverage summary counts are approximately accurate
 */

import { readdirSync, statSync, readFileSync } from 'fs';
import { join } from 'path';

const REPO_ROOT = process.cwd();
const DOC_PATH = join(REPO_ROOT, 'TEST_COVERAGE_ANALYSIS.md');
const TESTS_DIR = join(REPO_ROOT, 'ui/src/__tests__');
const FEATURES_DIR = join(REPO_ROOT, 'ui/src/features');

function getAllTestNames(): Set<string> {
  const testNames = new Set<string>();

  function scanDir(dir: string) {
    try {
      const entries = readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const fullPath = join(dir, entry.name);
        if (entry.isDirectory()) {
          scanDir(fullPath);
        } else if (entry.name.endsWith('.test.tsx') || entry.name.endsWith('.test.ts')) {
          const name = entry.name.replace(/\.test\.(tsx?|ts)$/, '');
          testNames.add(name);
        }
      }
    } catch (e) {
      // Directory might not exist
    }
  }

  scanDir(TESTS_DIR);
  return testNames;
}

function getAllFeatureComponentNames(): Set<string> {
  const componentNames = new Set<string>();

  function scanDir(dir: string) {
    try {
      const entries = readdirSync(dir, { withFileTypes: true });
      for (const entry of entries) {
        const fullPath = join(dir, entry.name);
        if (entry.isDirectory()) {
          if (entry.name !== '__tests__') {
            scanDir(fullPath);
          }
        } else if (entry.name.endsWith('.tsx') && entry.name !== 'register.tsx') {
          const name = entry.name.replace(/\.tsx$/, '');
          componentNames.add(name);
        }
      }
    } catch (e) {
      // Directory might not exist
    }
  }

  scanDir(FEATURES_DIR);
  return componentNames;
}

function readDoc(): string {
  return readFileSync(DOC_PATH, 'utf-8');
}

function parseWithoutTestsTables(doc: string): string[] {
  const components: string[] = [];

  // Find the section between "## 🚫 Components Without Test Coverage" and "## 🪝 Hooks Without Test Coverage"
  const startMarker = '## 🚫 Components Without Test Coverage';
  const endMarker = '## 🪝 Hooks Without Test Coverage';

  const startIdx = doc.indexOf(startMarker);
  const endIdx = doc.indexOf(endMarker);

  if (startIdx === -1 || endIdx === -1) return components;

  const section = doc.slice(startIdx + startMarker.length, endIdx);

  // Match table rows with component names in backticks
  // Pattern: | `ComponentName` | ...
  const tableRegex = /\|\s*`([^`]+)`\s*\|/g;
  let match;
  while ((match = tableRegex.exec(section)) !== null) {
    const componentName = match[1].trim();
    if (!componentName.includes('/') && !componentName.includes('.')) {
      components.push(componentName);
    }
  }

  return components;
}

function parseRecentlyCovered(doc: string): string[] {
  const components: string[] = [];

  // Find the section between "## ✅ Recently Covered" and the NEXT "## " heading
  const startMarker = '## ✅ Recently Covered';
  const startIdx = doc.indexOf(startMarker);
  if (startIdx === -1) return components;

  // Find the next ## heading after the start marker
  const sectionStart = startIdx + startMarker.length;
  const nextHeadingIdx = doc.indexOf('\n## ', sectionStart);
  const endIdx = nextHeadingIdx !== -1 ? nextHeadingIdx : doc.length;

  const section = doc.slice(sectionStart, endIdx);

  const tableRegex = /\|\s*`([^`]+)`\s*\|/g;
  let match;
  while ((match = tableRegex.exec(section)) !== null) {
    const componentName = match[1].trim();
    if (!componentName.includes('/') && !componentName.includes('.')) {
      components.push(componentName);
    }
  }

  return components;
}

function validate(): { passed: boolean; errors: string[]; warnings: string[] } {
  const errors: string[] = [];
  const warnings: string[] = [];

  const doc = readDoc();
  const testNames = getAllTestNames();
  const featureComponents = getAllFeatureComponentNames();

  // Parse document tables
  const withoutTestsComponents = parseWithoutTestsTables(doc);
  const recentlyCoveredComponents = parseRecentlyCovered(doc);

  console.log(`Found ${testNames.size} test files`);
  console.log(`Found ${featureComponents.size} feature components`);
  console.log(`Document lists ${withoutTestsComponents.length} components without tests`);
  console.log(`Document lists ${recentlyCoveredComponents.length} recently covered components`);

  // Check 1: Components in "Without Tests" should NOT have test files
  for (const comp of withoutTestsComponents) {
    if (testNames.has(comp)) {
      errors.push(`❌ "${comp}" is in "Without Tests" table BUT has a test file`);
    }
    if (!featureComponents.has(comp)) {
      warnings.push(`⚠️ "${comp}" in "Without Tests" not found in feature components`);
    }
  }

  // Check 2: Components in "Recently Covered" SHOULD have test files
  for (const comp of recentlyCoveredComponents) {
    if (!testNames.has(comp)) {
      errors.push(`❌ "${comp}" is in "Recently Covered" but NO test file found`);
    }
  }

  // Check 3: Any feature component with a test should NOT be in "Without Tests"
  for (const comp of featureComponents) {
    if (testNames.has(comp) && withoutTestsComponents.includes(comp)) {
      errors.push(`❌ "${comp}" has a test but is still listed in "Without Tests"`);
    }
  }

  // Check 4: Verify summary counts are roughly accurate
  const featureComponentsArray = Array.from(featureComponents);
  const actualWithoutTests = featureComponentsArray.filter(c => !testNames.has(c)).length;
  const docWithoutTestsMatch = withoutTestsComponents.length;

  if (Math.abs(actualWithoutTests - docWithoutTestsMatch) > 5) {
    warnings.push(`⚠️ Summary count mismatch: doc says ${docWithoutTestsMatch} without tests, reality is ~${actualWithoutTests}`);
  }

  return { passed: errors.length === 0, errors, warnings };
}

// Run validation
const result = validate();

console.log('\n=== VALIDATION RESULT ===');
if (result.passed) {
  console.log('✅ All checks passed!');
  if (result.warnings.length > 0) {
    console.log('\nWarnings:');
    result.warnings.forEach(w => console.log(w));
  }
  process.exit(0);
} else {
  console.log('❌ Validation failed:');
  result.errors.forEach(e => console.log(e));
  if (result.warnings.length > 0) {
    console.log('\nWarnings:');
    result.warnings.forEach(w => console.log(w));
  }
  process.exit(1);
}