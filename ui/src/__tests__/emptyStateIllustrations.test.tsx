/**
 * Tests for `EmptyStateIllustrations` — 15 inline SVG icon components.
 *
 * The icons are static SVG markup with a shared contract: currentColor,
 * aria-hidden, and configurable width/height. The resource-to-icon mapping
 * (EMPTY-09) is already tested in `emptyStateResourceIcons.test.tsx`, so
 * these tests focus on the rendering contract of each component.
 */

import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import {
  NoProductsIcon,
  NoSalesIcon,
  NoStaffIcon,
  NoShiftsIcon,
  NoCategoriesIcon,
  NoCustomersIcon,
  NoGiftCardsIcon,
  NoSuppliersIcon,
  NoPurchaseOrdersIcon,
  NoVariantsIcon,
  NoPromotionsIcon,
  NoLoyaltyIcon,
  NoTerminalsIcon,
  NotFoundIcon,
  EmptyBoxIcon,
} from '@/components/EmptyStateIllustrations';

const ALL_ICONS = [
  ['NoProductsIcon', NoProductsIcon],
  ['NoSalesIcon', NoSalesIcon],
  ['NoStaffIcon', NoStaffIcon],
  ['NoShiftsIcon', NoShiftsIcon],
  ['NoCategoriesIcon', NoCategoriesIcon],
  ['NoCustomersIcon', NoCustomersIcon],
  ['NoGiftCardsIcon', NoGiftCardsIcon],
  ['NoSuppliersIcon', NoSuppliersIcon],
  ['NoPurchaseOrdersIcon', NoPurchaseOrdersIcon],
  ['NoVariantsIcon', NoVariantsIcon],
  ['NoPromotionsIcon', NoPromotionsIcon],
  ['NoLoyaltyIcon', NoLoyaltyIcon],
  ['NoTerminalsIcon', NoTerminalsIcon],
  ['NotFoundIcon', NotFoundIcon],
  ['EmptyBoxIcon', EmptyBoxIcon],
] as const;

describe('empty state illustration icons', () => {
  it.each(ALL_ICONS)('%s renders an SVG with aria-hidden and currentColor', (name, Icon) => {
    const { container } = render(<Icon />);
    const svg = container.querySelector('svg');
    expect(svg, `${name} should render an <svg>`).toBeTruthy();
    expect(svg!.getAttribute('aria-hidden')).toBe('true');
    // The style attribute sets color via var(--color-fg-tertiary), but the
    // stroke attribute uses the literal 'currentColor' in the SVG markup.
    expect(svg!.getAttribute('stroke')).toBe('currentColor');
    // Default viewBox is 48x48.
    expect(svg!.getAttribute('viewBox')).toBe('0 0 48 48');
  });

  it.each(ALL_ICONS)('%s accepts custom width and height', (name, Icon) => {
    const { container } = render(<Icon width={64} height={96} />);
    const svg = container.querySelector('svg')!;
    expect(svg.getAttribute('width')).toBe('64');
    expect(svg.getAttribute('height')).toBe('96');
  });
});