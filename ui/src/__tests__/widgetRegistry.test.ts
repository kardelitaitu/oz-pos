/**
 * Tests for `widget-registry` — dashboard widget registration and
 * feature filtering.
 *
 * The dashboard renders widgets from this registry dynamically, so the
 * feature-gate filtering and duplicate-id overwrite semantics are the
 * contracts to pin.
 */

import { describe, expect, it, beforeEach } from 'vitest';
import {
  clearWidgets,
  getWidgets,
  registerWidget,
} from '@/platform/ui/widget-registry';

const widget = (id: string, extra: Partial<Parameters<typeof registerWidget>[0]> = {}) => ({
  id,
  component: () => null,
  title: id,
  ...extra,
});

describe('widget-registry', () => {
  beforeEach(() => clearWidgets());

  it('registers and lists widgets in registration order', () => {
    registerWidget(widget('a'));
    registerWidget(widget('b'));
    expect(getWidgets().map((w) => w.id)).toEqual(['a', 'b']);
  });

  it('overwrites a duplicate id with the last registration', () => {
    registerWidget(widget('a', { title: 'first' }));
    registerWidget(widget('a', { title: 'second' }));
    expect(getWidgets()).toHaveLength(1);
    expect(getWidgets()[0]!.title).toBe('second');
  });

  it('clearWidgets empties the registry', () => {
    registerWidget(widget('a'));
    clearWidgets();
    expect(getWidgets()).toHaveLength(0);
  });

  it('returns all widgets when enabledFeatures is omitted', () => {
    registerWidget(widget('a', { feature: 'pro' }));
    registerWidget(widget('b'));
    expect(getWidgets()).toHaveLength(2);
  });

  it('filters feature-gated widgets by the enabled set', () => {
    registerWidget(widget('a', { feature: 'pro' }));
    registerWidget(widget('b', { feature: 'base' }));
    registerWidget(widget('c')); // ungated — always shown
    const enabled = getWidgets(new Set(['base']));
    expect(enabled.map((w) => w.id)).toEqual(['b', 'c']);
  });

  it('shows all widgets when the enabled set does not contain the gate', () => {
    // A missing enabledFeatures means "everything enabled" (dashboard
    // default), NOT "nothing enabled".
    registerWidget(widget('a', { feature: 'pro' }));
    expect(getWidgets()).toHaveLength(1);
  });
});
