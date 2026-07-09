import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Badge } from '@/components/Badge';

describe('Badge', () => {
  it('renders children inside a span', () => {
    render(<Badge>Active</Badge>);
    const span = screen.getByText('Active');
    expect(span.tagName).toBe('SPAN');
  });

  it('applies default variant and size classes', () => {
    render(<Badge>Default</Badge>);
    const span = screen.getByText('Default');
    expect(span.className).toContain('badge--default');
    expect(span.className).toContain('badge--md');
  });

  it('applies the success variant class', () => {
    render(<Badge variant="success">Success</Badge>);
    const span = screen.getByText('Success');
    expect(span.className).toContain('badge--success');
  });

  it('applies the sm size class when size="sm"', () => {
    render(<Badge size="sm">Small</Badge>);
    const span = screen.getByText('Small');
    expect(span.className).toContain('badge--sm');
  });

  it('merges additional className prop', () => {
    render(<Badge className="my-extra">Extra</Badge>);
    const span = screen.getByText('Extra');
    expect(span.className).toContain('my-extra');
  });

  it('passes through additional HTML attributes', () => {
    render(<Badge data-testid="my-badge" aria-label="Status label">Tag</Badge>);
    const span = screen.getByTestId('my-badge');
    expect(span.getAttribute('aria-label')).toBe('Status label');
  });
});
