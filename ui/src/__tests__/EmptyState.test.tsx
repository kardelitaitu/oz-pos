import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { EmptyState } from '@/components/EmptyState';

vi.mock('@/components/Button', () => ({
  Button: ({ children, onClick, variant }: Record<string, unknown>) => (
    <button onClick={onClick as () => void} className={`btn btn--${variant as string || 'primary'}`}>
      {children as React.ReactNode}
    </button>
  ),
}));

describe('EmptyState', () => {
  it('renders the title as an h3', () => {
    render(<EmptyState title="Nothing here" />);
    const heading = screen.getByRole('heading', { level: 3 });
    expect(heading.textContent).toBe('Nothing here');
  });

  it('renders a description paragraph when provided', () => {
    render(<EmptyState title="Empty" description="Try adding an item to get started." />);
    expect(screen.getByText('Try adding an item to get started.')).toBeTruthy();
  });

  it('does not render a description element when not provided', () => {
    render(<EmptyState title="Empty" />);
    expect(document.querySelector('.empty-state__desc')).toBeNull();
  });

  it('renders an action button when action prop is provided', () => {
    const onClick = vi.fn();
    render(<EmptyState title="Empty" action={{ label: 'Add Item', onClick }} />);
    const btn = screen.getByText('Add Item');
    expect(btn).toBeTruthy();
    fireEvent.click(btn);
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it('does not render an action when not provided', () => {
    render(<EmptyState title="Empty" />);
    expect(document.querySelector('.empty-state__action')).toBeNull();
  });

  it('renders an icon when provided with aria-hidden', () => {
    render(<EmptyState title="Empty" icon={<span data-testid="icon">📦</span>} />);
    const icon = screen.getByTestId('icon');
    expect(icon).toBeTruthy();
    const iconWrapper = document.querySelector('.empty-state__icon');
    expect(iconWrapper).toBeTruthy();
    expect(iconWrapper!.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders additional children content', () => {
    render(
      <EmptyState title="Empty">
        <p data-testid="extra">Extra content</p>
      </EmptyState>,
    );
    expect(screen.getByTestId('extra')).toBeTruthy();
    expect(screen.getByTestId('extra').textContent).toBe('Extra content');
  });

  it('has role="status" on the container', () => {
    render(<EmptyState title="Status" />);
    expect(screen.getByRole('status')).toBeTruthy();
  });
});
