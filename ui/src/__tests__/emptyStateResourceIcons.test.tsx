import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { EmptyState } from '@/components/EmptyState';
import {
  EMPTY_STATE_RESOURCE_ICONS,
  emptyStateIconFor,
  type EmptyStateResource,
} from '@/components/emptyStateResourceIcons';
import { EmptyBoxIcon } from '@/components/EmptyStateIllustrations';

/** Renders the icon for a resource inside EmptyState and returns the SVG element. */
function renderIconFor(resource: EmptyStateResource | string) {
  const Icon = emptyStateIconFor(resource);
  render(<EmptyState title="Empty" icon={<Icon />} />);
  return document.querySelector('.empty-state svg');
}

describe('emptyStateIconFor (EMPTY-09)', () => {
  it('maps every declared resource type to an illustration component', () => {
    const resources: EmptyStateResource[] = [
      'products', 'sales', 'staff', 'shifts', 'categories', 'customers',
      'gift-cards', 'suppliers', 'purchase-orders', 'variants', 'promotions',
      'loyalty', 'terminals', 'search', 'generic',
    ];
    for (const r of resources) {
      expect(EMPTY_STATE_RESOURCE_ICONS[r], `missing mapping for ${r}`).toBeTypeOf('function');
    }
  });

  it('falls back to the generic box for unknown resources', () => {
    expect(emptyStateIconFor('unknown-resource')).toBe(EmptyBoxIcon);
    expect(emptyStateIconFor('')).toBe(EmptyBoxIcon);
  });

  it('returns the generic box itself for the generic resource', () => {
    expect(emptyStateIconFor('generic')).toBe(EmptyBoxIcon);
  });

  it('renders an accessible (aria-hidden) SVG for every resource', () => {
    const resources: EmptyStateResource[] = [
      'products', 'sales', 'staff', 'shifts', 'categories', 'customers',
      'gift-cards', 'suppliers', 'purchase-orders', 'variants', 'promotions',
      'loyalty', 'terminals', 'search',
    ];
    for (const r of resources) {
      renderIconFor(r);
      const svg = document.querySelector('.empty-state svg');
      expect(svg, `no svg rendered for ${r}`).toBeTruthy();
      expect(svg!.getAttribute('aria-hidden')).toBe('true');
      // Unmount between iterations to keep the DOM assertions independent.
      document.body.innerHTML = '';
    }
  });
});
