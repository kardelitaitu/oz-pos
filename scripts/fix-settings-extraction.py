#!/usr/bin/env python3
"""Replace inline case blocks in SettingsPage.tsx with section component calls."""

import re

path = r"C:\My Script\oz-pos\ui\src\features\settings\SettingsPage.tsx"

with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# ── Step 1: Add section imports after EmailReportSettings import ──────
section_imports = (
    "import EmailReportSettings from './EmailReportSettings';\n"
    "import GeneralSection from './sections/GeneralSection';\n"
    "import AppearanceSection from './sections/AppearanceSection';\n"
    "import ReceiptSection from './sections/ReceiptSection';\n"
    "import SyncSection from './sections/SyncSection';\n"
    "import AboutSection from './sections/AboutSection';"
)
old_import = "import EmailReportSettings from './EmailReportSettings';"
content = content.replace(old_import, section_imports, 1)

# ── Step 2: Remove unused imports ─────────────────────────────────────
# Remove AppearanceSettings import (now in AppearanceSection)
content = content.replace(
    "import { AppearanceSettings } from './AppearanceSettings';\n",
    ""
)
# Remove LanguageSelector import (now in GeneralSection)
content = content.replace(
    "import { LanguageSelector } from '@/i18n/LanguageSelector';\n",
    ""
)

# ── Step 3: Remove ExpiryInfo interface and formatTokenExpiry function ─
# These are now in SyncSection.tsx
expiry_pattern = re.compile(
    r'\n// ── Component .*?\n\n// ── Token expiry badge helper .*?\n\n'
    r'/\*\* Structured expiry info.*?\*/\n'
    r'interface ExpiryInfo \{.*?\}\n\n'
    r'/\*\* Compute a localisable expiry.*?\*/\n'
    r'function formatTokenExpiry.*?\n\}',
    re.DOTALL
)
content = expiry_pattern.sub('\n// ── Component ─────────────────────────────────────────────────────', content, count=1)

# ── Step 4: Replace the 5 inline case blocks ──────────────────────────

# General case: from "case 'general':" to just before "case 'appearance':"
general_new = (
    "      case 'general':\n"
    "        return (\n"
    "          <GeneralSection\n"
    "            store={store}\n"
    "            setStore={setStore}\n"
    "            markDirty={markDirty}\n"
    "            cmInput={cmInput}\n"
    "            fieldErrors={fieldErrors}\n"
    "            validateField={validateField}\n"
    "            clearFieldError={clearFieldError}\n"
    "            currencies={currencies}\n"
    "            defaultCurrency={defaultCurrency}\n"
    "            setDefaultCurrencyState={setDefaultCurrencyState}\n"
    "            l10n={l10n}\n"
    "          />\n"
    "        );\n"
)
general_pattern = re.compile(
    r"      case 'general':\n.*?"
    r"(?=\n      case 'appearance':)",
    re.DOTALL
)
content = general_pattern.sub(general_new, content, count=1)

# Appearance case: from "case 'appearance':" to just before "case 'receipt':"
appearance_new = (
    "      case 'appearance':\n"
    "        return (\n"
    "          <AppearanceSection\n"
    "            displayCardSize={displayCardSize}\n"
    "            setDisplayCardSize={setDisplayCardSize}\n"
    "            displayFontSize={displayFontSize}\n"
    "            setDisplayFontSize={setDisplayFontSize}\n"
    "            displayFontSmoothing={displayFontSmoothing}\n"
    "            setDisplayFontSmoothing={setDisplayFontSmoothing}\n"
    "            brandColour={brandColour}\n"
    "            setBrandColour={setBrandColour}\n"
    "            brandStoreName={brandStoreName}\n"
    "            setBrandStoreName={setBrandStoreName}\n"
    "            markDirty={markDirty}\n"
    "            l10n={l10n}\n"
    "          />\n"
    "        );\n"
)
appearance_pattern = re.compile(
    r"      case 'appearance':\n.*?"
    r"(?=\n      case 'receipt':)",
    re.DOTALL
)
content = appearance_pattern.sub(appearance_new, content, count=1)

# Receipt case: from "case 'receipt':" to just before "case 'sync':"
receipt_new = (
    "      case 'receipt':\n"
    "        return (\n"
    "          <ReceiptSection\n"
    "            receipt={receipt}\n"
    "            setReceipt={setReceipt}\n"
    "            setDecimalSep={setDecimalSep}\n"
    "            markDirty={markDirty}\n"
    "            l10n={l10n}\n"
    "          />\n"
    "        );\n"
)
receipt_pattern = re.compile(
    r"      case 'receipt':\n.*?"
    r"(?=\n      case 'sync':)",
    re.DOTALL
)
content = receipt_pattern.sub(receipt_new, content, count=1)

# Sync case: from "case 'sync':" to just before "case 'email':"
sync_new = (
    "      case 'sync':\n"
    "        return (\n"
    "          <SyncSection\n"
    "            sync={sync}\n"
    "            setSync={setSync}\n"
    "            syncServerUrl={syncServerUrl}\n"
    "            setSyncServerUrl={setSyncServerUrl}\n"
    "            syncApiKey={syncApiKey}\n"
    "            setSyncApiKey={setSyncApiKey}\n"
    "            syncApiKeyVisible={syncApiKeyVisible}\n"
    "            setSyncApiKeyVisible={setSyncApiKeyVisible}\n"
    "            syncing={syncing}\n"
    "            setSyncing={setSyncing}\n"
    "            pulling={pulling}\n"
    "            setPulling={setPulling}\n"
    "            syncResult={syncResult}\n"
    "            setSyncResult={setSyncResult}\n"
    "            pullResult={pullResult}\n"
    "            setPullResult={setPullResult}\n"
    "            pendingCount={pendingCount}\n"
    "            testing={testing}\n"
    "            setTesting={setTesting}\n"
    "            pingResult={pingResult}\n"
    "            setPingResult={setPingResult}\n"
    "            requesting={requesting}\n"
    "            setRequesting={setRequesting}\n"
    "            tokenExpiresAt={tokenExpiresAt}\n"
    "            setTokenExpiresAt={setTokenExpiresAt}\n"
    "            cmInput={cmInput}\n"
    "            markDirty={markDirty}\n"
    "            refreshPendingCount={refreshPendingCount}\n"
    "            testSyncConnection={testSyncConnection}\n"
    "            syncRun={syncRun}\n"
    "            syncPull={syncPull}\n"
    "            requestSyncToken={requestSyncToken}\n"
    "            l10n={l10n}\n"
    "            addToast={addToast}\n"
    "          />\n"
    "        );\n"
)
sync_pattern = re.compile(
    r"      case 'sync':\n.*?"
    r"(?=\n      case 'email':)",
    re.DOTALL
)
content = sync_pattern.sub(sync_new, content, count=1)

# About case: from "case 'about':" to just before "case 'license':"
about_new = (
    "      case 'about':\n"
    "        return (\n"
    "          <AboutSection\n"
    "            appVersion={appVersion}\n"
    "            updateState={updateState}\n"
    "            updateVersion={updateVersion}\n"
    "            handleCheckUpdates={handleCheckUpdates}\n"
    "            handleInstallUpdate={handleInstallUpdate}\n"
    "          />\n"
    "        );\n"
)
about_pattern = re.compile(
    r"      case 'about':\n.*?"
    r"(?=\n      case 'license':)",
    re.DOTALL
)
content = about_pattern.sub(about_new, content, count=1)

# ── Step 5: Also remove unused SettingsSelect import? No — check first ──
# SettingsSelect is no longer used in SettingsPage after extraction.
# But let's be safe and only remove things we're sure about.
# SettingsSelect is only used inline in general/appearance/receipt which are now extracted.
content = content.replace(
    "import SettingsSelect from './SettingsSelect';\n",
    ""
)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)

print(f"Done. Written to {path}")
print(f"Line count after: {len(content.splitlines())}")
