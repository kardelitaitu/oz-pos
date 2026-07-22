import { createRef } from 'react';
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Button } from '../components/Button';

describe('Button', () => {
  it('renders children text', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText('Click me')).not.toBeNull();
  });

  it('defaults to type="button"', () => {
    render(<Button>Save</Button>);
    const btn = screen.getByRole('button');
    expect(btn.getAttribute('type')).toBe('button');
  });

  it('renders primary variant by default', () => {
    render(<Button>Primary</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--primary');
  });

  it('renders secondary variant', () => {
    render(<Button variant="secondary">Secondary</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--secondary');
  });

  it('renders danger variant', () => {
    render(<Button variant="danger">Delete</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--danger');
  });

  it('renders ghost variant', () => {
    render(<Button variant="ghost">Cancel</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--ghost');
  });

  it('renders md size by default', () => {
    render(<Button>Medium</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--md');
  });

  it('renders sm size', () => {
    render(<Button size="sm">Small</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--sm');
  });

  it('renders lg size', () => {
    render(<Button size="lg">Large</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--lg');
  });

  it('renders icon-only button with modifier class', () => {
    render(
      <Button iconOnly aria-label="Close">
        <span data-testid="icon">×</span>
      </Button>,
    );
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--icon-only');
  });

  it('renders unstyled button without variant/size classes', () => {
    render(
      <Button unstyled aria-label="Toggle">
        <span data-testid="icon">☰</span>
      </Button>,
    );
    const btn = screen.getByRole('button');
    expect(btn.className).not.toContain('btn--primary');
    expect(btn.className).not.toContain('btn--md');
    expect(btn.className).not.toContain('btn--ghost');
    expect(btn.className).toContain('btn--unstyled');
  });

  it('preserves base button class with unstyled=false by default', () => {
    render(<Button>Default</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn');
    expect(btn.className).not.toContain('btn--unstyled');
  });

  it('applies icon-only class alongside variant and size classes', () => {
    render(
      <Button variant="ghost" size="sm" iconOnly aria-label="Close">
        <span>×</span>
      </Button>,
    );
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn--ghost');
    expect(btn.className).toContain('btn--sm');
    expect(btn.className).toContain('btn--icon-only');
  });

  it('disables button and shows spinner when loading', () => {
    render(<Button loading>Loading</Button>);
    const btn = screen.getByRole('button');
    expect(btn.getAttribute('disabled')).not.toBeNull();
    expect(btn.getAttribute('aria-busy')).toBe('true');
    const spinner = btn.querySelector('.btn__spinner');
    expect(spinner).not.toBeNull();
  });

  it('renders icon when provided and not loading', () => {
    render(<Button icon={<span data-testid="test-icon">⚡</span>}>With Icon</Button>);
    const btn = screen.getByRole('button');
    const icon = btn.querySelector('.btn__icon');
    expect(icon).not.toBeNull();
    expect(screen.getByTestId('test-icon')).not.toBeNull();
  });

  it('does not render icon when loading', () => {
    render(<Button loading icon={<span>⚡</span>}>Loading</Button>);
    const btn = screen.getByRole('button');
    expect(btn.querySelector('.btn__icon')).toBeNull();
    expect(btn.querySelector('.btn__spinner')).not.toBeNull();
  });

  it('applies disabled attribute', () => {
    render(<Button disabled>Disabled</Button>);
    const btn = screen.getByRole('button');
    expect(btn.getAttribute('disabled')).not.toBeNull();
    expect(btn.getAttribute('aria-disabled')).not.toBeNull();
  });

  it('accepts custom className', () => {
    render(<Button className="custom-btn">Custom</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('custom-btn');
  });

  it('accepts custom type', () => {
    render(<Button type="submit">Submit</Button>);
    const btn = screen.getByRole('button');
    expect(btn.getAttribute('type')).toBe('submit');
  });

  it('spreads extra HTML attributes', () => {
    render(<Button data-testid="my-btn" aria-label="Close">X</Button>);
    const btn = screen.getByTestId('my-btn');
    expect(btn.getAttribute('aria-label')).toBe('Close');
  });

  it('always includes btn base class', () => {
    render(<Button>Base</Button>);
    const btn = screen.getByRole('button');
    expect(btn.className).toContain('btn');
  });

  it('supports forwardRef', () => {
    const ref = createRef<HTMLButtonElement>();
    render(<Button ref={ref}>Focusable</Button>);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
    expect(ref.current?.textContent).toBe('Focusable');
  });

  it('disables button when both disabled and loading', () => {
    render(<Button disabled loading>Double</Button>);
    const btn = screen.getByRole('button');
    expect(btn.getAttribute('disabled')).not.toBeNull();
    expect(btn.getAttribute('aria-busy')).toBe('true');
  });

  describe('state prop', () => {
    it('defaults to ready', () => {
      render(<Button>Save</Button>);
      const btn = screen.getByRole('button');
      expect(btn.getAttribute('disabled')).toBeNull();
      expect(btn.getAttribute('aria-busy')).toBeNull();
    });

    it('shows spinner and disables button when processing', () => {
      render(<Button state="processing">Saving</Button>);
      const btn = screen.getByRole('button');
      expect(btn.getAttribute('disabled')).not.toBeNull();
      expect(btn.getAttribute('aria-busy')).toBe('true');
      const spinner = btn.querySelector('.btn__spinner');
      expect(spinner).not.toBeNull();
    });

    it('hides icon when processing', () => {
      render(<Button state="processing" icon={<span>⚡</span>}>Saving</Button>);
      const btn = screen.getByRole('button');
      expect(btn.querySelector('.btn__icon')).toBeNull();
      expect(btn.querySelector('.btn__spinner')).not.toBeNull();
    });

    it('wraps children in sr-only span when processing', () => {
      render(<Button state="processing">Saving</Button>);
      const btn = screen.getByRole('button');
      const srSpan = btn.querySelector('.sr-only');
      expect(srSpan).not.toBeNull();
      expect(srSpan?.textContent).toBe('Saving');
    });

    it('adds no extra CSS class when processing', () => {
      render(<Button state="processing">Saving</Button>);
      const btn = screen.getByRole('button');
      expect(btn.className).not.toContain('processing');
    });

    it('spinner has aria-hidden="true"', () => {
      render(<Button state="processing">Saving</Button>);
      const spinner = screen.getByRole('button').querySelector('.btn__spinner');
      expect(spinner?.getAttribute('aria-hidden')).toBe('true');
    });

    it('works when both state="ready" and loading are set (loading wins)', () => {
      render(<Button state="ready" loading>Save</Button>);
      const btn = screen.getByRole('button');
      expect(btn.getAttribute('disabled')).not.toBeNull();
      expect(btn.getAttribute('aria-busy')).toBe('true');
      expect(btn.querySelector('.btn__spinner')).not.toBeNull();
    });

    it('works with disabled and state="processing"', () => {
      render(<Button disabled state="processing">Saving</Button>);
      const btn = screen.getByRole('button');
      expect(btn.getAttribute('disabled')).not.toBeNull();
      expect(btn.getAttribute('aria-busy')).toBe('true');
    });
  });
});
