/* eslint-disable jsx-a11y/label-has-associated-control -- <Localized> blocks static analysis */
import { useState, useCallback } from 'react';
import { Button } from '@/components/Button';
import { Card } from '@/components/Card';
import { Localized } from '@/frontend/shared/Localized';
import { useLocalization } from '@fluent/react';
import './SetupWizard.css';

// ── Types ──────────────────────────────────────────────────────────

export type Preset = 'simple-retail' | 'restaurant' | 'full-store' | 'custom';

export interface FeatureDef {
  key: string;
  label: string;
  description: string;
}

export interface WizardState {
  preset: Preset | null;
  features: Record<string, boolean>;
}

// ── Constants ──────────────────────────────────────────────────────

const STEPS = [
  'Store Type',
  'Payments',
  'Products',
  'Staff',
  'Hardware',
  'Business Rules',
  'Data & Cloud',
  'Review',
] as const;

export const TOTAL_STEPS = STEPS.length;

/** Suffixes used to build the FTL id for each step (e.g. `setup-step-store-type`). */
const STEP_IDS = [
  'store-type',
  'payments',
  'products',
  'staff',
  'hardware',
  'business-rules',
  'data-cloud',
  'review',
] as const;

interface PresetOption {
  value: Preset;
  emoji: string;
  name: string;
  description: string;
}

const PRESETS: PresetOption[] = [
  {
    value: 'simple-retail',
    emoji: '🛒',
    name: 'Simple Retail',
    description: 'Barcode scan, cash, receipt, inventory, tax — all essentials',
  },
  {
    value: 'restaurant',
    emoji: '🍽️',
    name: 'Restaurant',
    description: 'Tables, KDS, discounts, staff login — built for dining',
  },
  {
    value: 'full-store',
    emoji: '🏪',
    name: 'Full Store',
    description: 'Everything except cloud — payments, staff, loyalty, reports',
  },
  {
    value: 'custom',
    emoji: '⚙️',
    name: 'Custom',
    description: 'Start from scratch — enable exactly what you need',
  },
];

/** Features shown per wizard step. */
const STEP_FEATURES: { sectionId: string; title: string; features: FeatureDef[] }[] = [
  // Step 2 — Payments
  {
    sectionId: 'payments',
    title: 'Payment Methods',
    features: [
      { key: 'cash-payment', label: 'Cash', description: 'Accept cash payments and track cash drawer' },
      { key: 'card-payment', label: 'Card', description: 'Accept debit and credit card payments' },
      { key: 'multi-currency', label: 'Multi-Currency', description: 'Support multiple currencies with exchange rates' },
    ],
  },
  // Step 3 — Products
  {
    sectionId: 'products',
    title: 'Products & Inventory',
    features: [
      { key: 'inventory-tracking', label: 'Inventory Tracking', description: 'Track stock levels per product with alerts' },
      { key: 'product-variants', label: 'Product Variants', description: 'Size, colour, flavour variants per product' },
      { key: 'categories-enabled', label: 'Categories', description: 'Group products by category with colour coding' },
    ],
  },
  // Step 4 — Staff
  {
    sectionId: 'staff',
    title: 'Staff Management',
    features: [
      { key: 'staff-login', label: 'Staff Login', description: 'PIN or password login for cashiers' },
      { key: 'staff-roles', label: 'Staff Roles', description: 'Owner, manager, cashier permission levels' },
      { key: 'shift-management', label: 'Shift Management', description: 'Open/close shifts with cash reconciliation' },
      { key: 'audit-log', label: 'Audit Log', description: 'Immutable log of sensitive actions' },
    ],
  },
  // Step 5 — Hardware
  {
    sectionId: 'hardware',
    title: 'Hardware & Peripherals',
    features: [
      { key: 'barcode-scanning', label: 'Barcode Scanner', description: 'USB, serial, or Bluetooth barcode scanning' },
      { key: 'receipt-printing', label: 'Receipt Printer', description: 'USB, serial, or network receipt printing' },
      { key: 'cash-drawer', label: 'Cash Drawer', description: 'Automatic cash drawer via printer GPIO' },
      { key: 'customer-display', label: 'Customer Display', description: 'Secondary display facing the customer' },
      { key: 'nfc-reader', label: 'NFC Reader', description: 'Contactless payment and loyalty card reading' },
    ],
  },
  // Step 6 — Business Rules
  {
    sectionId: 'business-rules',
    title: 'Business Rules',
    features: [
      { key: 'discount-engine', label: 'Discounts', description: 'Percentage and fixed-amount discounts on items or cart' },
      { key: 'tax-engine', label: 'Tax Engine', description: 'Tax inclusive/exclusive with configurable rates' },
      { key: 'loyalty-program', label: 'Loyalty Program', description: 'Customer points, tiers, and rewards' },
      { key: 'promotions-engine', label: 'Promotions', description: 'Buy-X-get-Y, time-limited offers, bundles' },
      { key: 'product-bundles', label: 'Product Bundles', description: 'Sell multiple SKUs together as a single item' },
    ],
  },
  // Step 7 — Data & Cloud
  {
    sectionId: 'data-cloud',
    title: 'Data, Reporting & Cloud',
    features: [
      { key: 'reporting', label: 'Reporting', description: 'Sales, inventory, and shift reports' },
      { key: 'analytics', label: 'Analytics', description: 'Charts, top products, hourly heatmap, CSV exports' },
      { key: 'export-import', label: 'Export & Import', description: 'Encrypted data export and import (.ozpkg)' },
      { key: 'cloud-sync', label: 'Cloud Sync', description: 'Sync data to cloud PostgreSQL with backup' },
      { key: 'multi-store', label: 'Multi-Store', description: 'Manage multiple store locations' },
      { key: 'multi-terminal', label: 'Multi-Terminal', description: 'Multiple POS terminals per store' },
      { key: 'plugin-system', label: 'Plugin System', description: 'Third-party plugins and custom drivers' },
    ],
  },
];

/** Feature keys enabled by default for each preset. */
const PRESET_FEATURES: Record<Preset, string[]> = {
  'simple-retail': [
    'barcode-scanning',
    'receipt-printing',
    'cash-payment',
    'inventory-tracking',
    'categories-enabled',
    'tax-engine',
  ],
  restaurant: [
    'receipt-printing',
    'cash-payment',
    'inventory-tracking',
    'categories-enabled',
    'discount-engine',
    'tax-engine',
    'staff-login',
  ],
  'full-store': [
    'cash-payment',
    'card-payment',
    'multi-currency',
    'inventory-tracking',
    'product-variants',
    'categories-enabled',
    'staff-login',
    'staff-roles',
    'shift-management',
    'audit-log',
    'barcode-scanning',
    'receipt-printing',
    'cash-drawer',
    'customer-display',
    'nfc-reader',
    'discount-engine',
    'tax-engine',
    'loyalty-program',
    'promotions-engine',
    'product-bundles',
    'reporting',
    'analytics',
    'export-import',
  ],
  custom: [],
};

/** Human-readable labels for review summary. */
const FEATURE_LABELS: Record<string, string> = {
  'cash-payment': 'Cash',
  'card-payment': 'Card',
  'multi-currency': 'Multi-Currency',
  'inventory-tracking': 'Inventory',
  'product-variants': 'Variants',
  'categories-enabled': 'Categories',
  'staff-login': 'Staff Login',
  'staff-roles': 'Staff Roles',
  'shift-management': 'Shifts',
  'audit-log': 'Audit Log',
  'barcode-scanning': 'Barcode',
  'receipt-printing': 'Receipts',
  'cash-drawer': 'Cash Drawer',
  'customer-display': 'Customer Display',
  'nfc-reader': 'NFC',
  'discount-engine': 'Discounts',
  'tax-engine': 'Tax',
  'loyalty-program': 'Loyalty',
  'promotions-engine': 'Promotions',
  'product-bundles': 'Bundles',
  'reporting': 'Reports',
  'analytics': 'Analytics',
  'export-import': 'Export/Import',
  'cloud-sync': 'Cloud Sync',
  'multi-store': 'Multi-Store',
  'multi-terminal': 'Multi-Terminal',
  'plugin-system': 'Plugins',
};

const PRESET_NAMES: Record<Preset, string> = {
  'simple-retail': 'Simple Retail',
  restaurant: 'Restaurant',
  'full-store': 'Full Store',
  custom: 'Custom',
};

// ── Props ──────────────────────────────────────────────────────────

export interface SetupWizardProps {
  /** Called when the wizard completes with the chosen state. */
  onComplete?: (state: WizardState) => void;
  /** Called to skip the wizard entirely. */
  onSkip?: () => void;
}

// ── Component ──────────────────────────────────────────────────────

/**
 * 8-step first-run Setup Wizard.
 *
 * Steps:
 *   1. Store Type / Preset  5. Hardware
 *   2. Payments             6. Business Rules
 *   3. Products             7. Data & Cloud
 *   4. Staff                8. Review & Confirm
 */
export default function SetupWizard({ onComplete, onSkip }: SetupWizardProps) {
  const { l10n } = useLocalization();
  const [step, setStep] = useState(0);
  const [preset, setPreset] = useState<Preset | null>(null);
  const [features, setFeatures] = useState<Record<string, boolean>>({});
  const [completed, setCompleted] = useState(false);

  // ── Preset selection ────────────────────────────────────────────
  const handleSelectPreset = useCallback((p: Preset) => {
    setPreset(p);
    // Convert the array of feature keys to a boolean record.
    const enabled: Record<string, boolean> = {};
    for (const key of PRESET_FEATURES[p]) {
      enabled[key] = true;
    }
    setFeatures(enabled);
    // Advance to step 2 so the user can customise.
    setStep(1);
  }, []);

  // ── Feature toggle ──────────────────────────────────────────────
  const toggleFeature = useCallback((key: string) => {
    setFeatures((prev) => ({ ...prev, [key]: !prev[key] }));
  }, []);

  // ── Navigation ──────────────────────────────────────────────────
  const goNext = useCallback(() => {
    if (step < TOTAL_STEPS - 1) {
      setStep((s) => s + 1);
    }
  }, [step]);

  const goBack = useCallback(() => {
    if (step > 0) {
      setStep((s) => s - 1);
    }
  }, [step]);

  const handleComplete = useCallback(() => {
    setCompleted(true);
    if (onComplete && preset) {
      onComplete({ preset, features });
    }
  }, [onComplete, preset, features]);
  // ── All-features list for review ─────────────────────────────────

  const allFeatures = STEP_FEATURES.flatMap((s) => s.features);
  const enabledCount = Object.values(features).filter(Boolean).length;

  // ── Step indicator dots ──────────────────────────────────────────
  const stepIndicator = (
    <Localized id="setup-progress-aria" attrs={{ 'aria-label': true }}>
      <nav className="setup-progress" aria-label="Setup progress">
        {STEPS.map((label, i) => {
          const stepLabel = l10n.getString(`setup-step-${STEP_IDS[i]}`) || label;
          return (
            <span key={label} className="setup-step-group">
              <span
                className={
                  i === step
                    ? 'setup-step-dot setup-step-dot--active'
                    : i < step || (completed && i === TOTAL_STEPS - 1)
                      ? 'setup-step-dot setup-step-dot--completed'
                      : 'setup-step-dot setup-step-dot--pending'
                }
                aria-current={i === step ? 'step' : undefined}
                aria-label={l10n.getString('setup-step-aria', { number: i + 1, label: stepLabel }) || `Step ${i + 1}: ${label}`}
              >
                {i < step || (completed && i === TOTAL_STEPS - 1) ? '✓' : i + 1}
              </span>
              {i < TOTAL_STEPS - 1 && (
                <span
                  className={
                    i < step
                      ? 'setup-step-line setup-step-line--completed'
                      : 'setup-step-line setup-step-line--pending'
                  }
                  aria-hidden="true"
                />
              )}
            </span>
          );
        })}
      </nav>
    </Localized>
  );

  // ── Completion screen ───────────────────────────────────────────
  if (completed) {
    return (
      <div className="setup-page">
        <div className="setup-container">
          <div className="setup-header">
            <Localized id="setup-logo">
              <div className="setup-logo">OZ-POS</div>
            </Localized>
          </div>

          {stepIndicator}

          <div className="setup-complete">
            <div className="setup-complete-check" aria-hidden="true">
              <svg viewBox="0 0 24 24">
                <polyline points="20 6 9 17 4 12" />
              </svg>
            </div>
            <Localized id="setup-complete-title">
              <h1 className="setup-complete-title">All Set!</h1>
            </Localized>
            <p className="setup-complete-desc">
              {l10n.getString('setup-complete-desc', {
                preset: preset ? l10n.getString(`setup-preset-${preset}`) || PRESET_NAMES[preset] : l10n.getString('setup-preset-custom'),
              })}
            </p>
            <p className="setup-complete-features">
              {l10n.getString('setup-complete-features', { count: enabledCount })}
            </p>
            <Button size="lg" onClick={onSkip}>
              <Localized id="setup-launch">Launch OZ-POS</Localized>
            </Button>
          </div>
        </div>
      </div>
    );
  }

  // ── Navigation footer ────────────────────────────────────────────
  const showBack = step > 0;
  const showSkip = step === 0 && !preset;
  const isLastStep = step === TOTAL_STEPS - 1;

  return (
    <div className="setup-page">
      <div className="setup-container">
        {/* ── Header ──────────────────────────────── */}
        <div className="setup-header">
          <Localized id="setup-logo">
            <div className="setup-logo">OZ-POS</div>
          </Localized>
          <Localized id="setup-tagline">
            <div className="setup-tagline">Point of Sale — Simplified</div>
          </Localized>
        </div>

        {stepIndicator}

        {/* ── Step content ────────────────────────── */}
        <div className="setup-step-content" key={step}>
          <div className="setup-step-panel">
            {step === 0 && (
              <StepPreset
                selected={preset}
                onSelect={handleSelectPreset}
              />
            )}

            {step >= 1 && step <= 6 && (
              <StepFeatures
                sectionId={STEP_FEATURES[step - 1]!.sectionId}
                title={STEP_FEATURES[step - 1]!.title}
                features={STEP_FEATURES[step - 1]!.features}
                enabled={features}
                onToggle={toggleFeature}
              />
            )}

            {step === 7 && (
              <StepReview
                preset={preset}
                features={features}
                allFeatures={allFeatures}
              />
            )}
          </div>
        </div>

        {/* ── Navigation ──────────────────────────── */}
        <div className="setup-nav">
          <div className="setup-nav-left">
            {showBack && (
              <Button variant="ghost" onClick={goBack}>
                <Localized id="setup-back">Back</Localized>
              </Button>
            )}
          </div>

          <div className="setup-nav-right">
            {showSkip && onSkip && (
              <button
                type="button"
                className="setup-skip-btn"
                onClick={onSkip}
              >
                <Localized id="setup-skip">Skip setup</Localized>
              </button>
            )}

            {isLastStep ? (
              <Button
                variant="primary"
                size="lg"
                onClick={handleComplete}
                disabled={!preset}
              >
                <Localized id="setup-finish">Complete Setup</Localized>
              </Button>
            ) : (
              step > 0 && (
                <Button variant="primary" onClick={goNext}>
                  <Localized id="setup-next">Next</Localized>
                </Button>
              )
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Step 1: Preset Selection ────────────────────────────────────────

function StepPreset({
  selected,
  onSelect,
}: {
  selected: Preset | null;
  onSelect: (p: Preset) => void;
}) {
  return (
    <>
      <Localized id="setup-preset-question">
        <h2 className="setup-step-title">What kind of store are you running?</h2>
      </Localized>
      <Localized id="setup-preset-desc">
        <p className="setup-step-desc">
          Choose a preset to get started quickly, or customise every feature later.
        </p>
      </Localized>

      <Localized id="setup-preset-group-aria" attrs={{ 'aria-label': true }}>
        <div className="setup-presets" role="radiogroup" aria-label="Store preset">
          {PRESETS.map((p) => (
            <button
              key={p.value}
              type="button"
              role="radio"
              aria-checked={selected === p.value}
              className={
                selected === p.value
                  ? 'setup-preset-card setup-preset-card--selected'
                  : 'setup-preset-card'
              }
              onClick={() => onSelect(p.value)}
            >
              <span className="setup-preset-emoji" aria-hidden="true">
                {p.emoji}
              </span>
              <Localized id={`setup-preset-${p.value}`}>
                <span className="setup-preset-name">{p.name}</span>
              </Localized>
              <Localized id={`setup-preset-${p.value}-desc`}>
                <span className="setup-preset-desc">{p.description}</span>
              </Localized>
            </button>
          ))}
        </div>
      </Localized>
    </>
  );
}

// ── Steps 2–7: Feature Toggles ──────────────────────────────────────

function StepFeatures({
  sectionId,
  title,
  features,
  enabled,
  onToggle,
}: {
  sectionId: string;
  title: string;
  features: FeatureDef[];
  enabled: Record<string, boolean>;
  onToggle: (key: string) => void;
}) {
  const { l10n } = useLocalization();
  const localizedTitle = l10n.getString(`setup-features-section-${sectionId}`) || title;

  return (
    <>
      <Localized id="setup-features-title" vars={{ title: localizedTitle }}>
        <h2 className="setup-step-title">{title}</h2>
      </Localized>
      <Localized id="setup-features-desc">
        <p className="setup-step-desc">
          Toggle the features you need. You can change these later.
        </p>
      </Localized>

      <Localized id="setup-features-group-aria" attrs={{ 'aria-label': true }} vars={{ title: localizedTitle }}>
        <div className="setup-features" role="group" aria-label={title}>
          {features.map((f) => {
            const isOn = !!enabled[f.key];
            const label = l10n.getString(`setup-feature-${f.key}-label`) || l10n.getString(`setup-feature-${f.key}`) || f.label;
            return (
              <label
                key={f.key}
                className="setup-feature-row"
              >
                <div className="setup-feature-info">
                  <Localized id={`setup-feature-${f.key}-label`}>
                    <div className="setup-feature-name">{f.label}</div>
                  </Localized>
                  <Localized id={`setup-feature-${f.key}-desc`}>
                    <div className="setup-feature-desc">{f.description}</div>
                  </Localized>
                </div>

                <span className="toggle-switch">
                  <Localized id="setup-features-toggle-aria" attrs={{ 'aria-label': true }} vars={{ label }}>
                    <input
                      type="checkbox"
                      checked={isOn}
                      onChange={() => onToggle(f.key)}
                      aria-label={`Toggle ${f.label}`}
                    />
                  </Localized>
                  <span className="toggle-track">
                    <span className="toggle-thumb" />
                  </span>
                </span>
              </label>
            );
          })}
        </div>
      </Localized>
    </>
  );
}

// ── Step 8: Review & Confirm ───────────────────────────────────────

function StepReview({
  preset,
  features,
  allFeatures,
}: {
  preset: Preset | null;
  features: Record<string, boolean>;
  allFeatures: FeatureDef[];
}) {
  const { l10n } = useLocalization();
  const enabledFeatures = allFeatures.filter((f) => features[f.key]);
  const disabledFeatures = allFeatures.filter((f) => !features[f.key]);

  return (
    <>
      <Localized id="setup-review-title">
        <h2 className="setup-step-title">Review Your Setup</h2>
      </Localized>
      <Localized id="setup-review-desc">
        <p className="setup-step-desc">
          Here&rsquo;s a summary of your configuration. You can change anything later.
        </p>
      </Localized>

      <div className="setup-review-list">
        {/* Preset summary */}
        <Card padding="md" shadow="sm">
          <p className="setup-preset-summary">
            {l10n.getString('setup-review-preset', {
              name: preset ? l10n.getString(`setup-preset-${preset}`) || PRESET_NAMES[preset] : l10n.getString('setup-review-none') || 'None',
            })}
          </p>
        </Card>

        {/* Enabled features */}
        <div className="setup-review-section">
          <h3 className="setup-review-section-title">
            {l10n.getString('setup-review-enabled', { count: enabledFeatures.length })}
          </h3>
          <div className="setup-tag-cloud">
            {enabledFeatures.length === 0 ? (
              <Localized id="setup-review-none">
                <span className="setup-tag setup-tag--disabled">None</span>
              </Localized>
            ) : (
              enabledFeatures.map((f) => (
                <Localized key={f.key} id={`setup-feature-${f.key}`}>
                  <span className="setup-tag setup-tag--enabled">
                    {FEATURE_LABELS[f.key] ?? f.label}
                  </span>
                </Localized>
              ))
            )}
          </div>
        </div>

        {/* Disabled features */}
        <div className="setup-review-section">
          <h3 className="setup-review-section-title">
            {l10n.getString('setup-review-disabled', { count: disabledFeatures.length })}
          </h3>
          <div className="setup-tag-cloud">
            {disabledFeatures.length === 0 ? (
              <Localized id="setup-review-all-on">
                <span className="setup-tag setup-tag--enabled">Everything on!</span>
              </Localized>
            ) : (
              disabledFeatures.slice(0, 20).map((f) => (
                <Localized key={f.key} id={`setup-feature-${f.key}`}>
                  <span className="setup-tag setup-tag--disabled">
                    {FEATURE_LABELS[f.key] ?? f.label}
                  </span>
                </Localized>
              ))
            )}
            {disabledFeatures.length > 20 && (
              <span className="setup-tag setup-tag--disabled">
                {l10n.getString('setup-review-more', { count: disabledFeatures.length - 20 })}
              </span>
            )}
          </div>
        </div>
      </div>
    </>
  );
}
