/**
 * Widget Registry — modules register dashboard widgets here so the
 * dashboard screen can render them dynamically.
 *
 * @example
 * ```tsx
 * import { registerWidget } from '@/platform/ui/widget-registry';
 * import SalesSummaryWidget from './SalesSummaryWidget';
 *
 * registerWidget({
 *   id: 'sales-summary',
 *   component: SalesSummaryWidget,
 *   title: 'Sales Summary',
 *   feature: 'simple-retail',
 * });
 * ```
 */

import type { ComponentType } from 'react';

// ── Types ──────────────────────────────────────────────────────────

export interface WidgetRegistration {
  /** Unique widget identifier. */
  id: string;
  /** The React component to render. */
  component: ComponentType;
  /** Display title for the widget card. */
  title: string;
  /** Optional feature key that must be enabled for this widget to appear. */
  feature?: string;
  /** Optional grid width (in columns, default 1). */
  width?: 1 | 2 | 3;
  /** Optional grid height (in rows, default 1). */
  height?: 1 | 2;
}

// ── Registry ───────────────────────────────────────────────────────

const widgets = new Map<string, WidgetRegistration>();

/**
 * Register a widget. Duplicate IDs will be overwritten.
 */
export function registerWidget(registration: WidgetRegistration): void {
  widgets.set(registration.id, registration);
}

/**
 * Get all registered widgets, optionally filtered by enabled features.
 */
export function getWidgets(
  enabledFeatures?: Set<string>,
): WidgetRegistration[] {
  return Array.from(widgets.values()).filter((w) => {
    if (!w.feature) return true;
    if (!enabledFeatures) return true;
    return enabledFeatures.has(w.feature);
  });
}

/**
 * Clear all registrations (useful for testing).
 */
export function clearWidgets(): void {
  widgets.clear();
}
