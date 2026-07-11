import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { Skeleton } from '../components/Skeleton';

describe('Skeleton', () => {
  it('renders with base skeleton class', () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.classList.contains('skeleton')).toBe(true);
  });

  it('defaults to text variant', () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.classList.contains('skeleton--text')).toBe(true);
  });

  it('renders circle variant', () => {
    const { container } = render(<Skeleton variant="circle" />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.classList.contains('skeleton--circle')).toBe(true);
  });

  it('renders block variant', () => {
    const { container } = render(<Skeleton variant="block" />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.classList.contains('skeleton--block')).toBe(true);
  });

  it('has aria-hidden for screen readers', () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.getAttribute('aria-hidden')).toBe('true');
  });

  it('accepts custom className', () => {
    const { container } = render(<Skeleton className="my-skeleton" />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.classList.contains('my-skeleton')).toBe(true);
    expect(el.classList.contains('skeleton')).toBe(true);
  });

  it('applies custom width via style', () => {
    const { container } = render(<Skeleton width="200px" />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.style.width).toBe('200px');
  });

  it('applies custom height via style', () => {
    const { container } = render(<Skeleton height="1em" />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.style.height).toBe('1em');
  });

  it('applies both width and height', () => {
    const { container } = render(<Skeleton width="100%" height="40px" />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.style.width).toBe('100%');
    expect(el.style.height).toBe('40px');
  });

  it('merges custom style with width/height', () => {
    const { container } = render(
      <Skeleton width="200px" style={{ marginTop: '8px', borderRadius: '4px' }} />,
    );
    const el = container.firstElementChild as HTMLElement;
    expect(el.style.width).toBe('200px');
    expect(el.style.marginTop).toBe('8px');
    expect(el.style.borderRadius).toBe('4px');
  });

  it('spreads extra HTML attributes', () => {
    const { container } = render(<Skeleton data-testid="loading-skeleton" />);
    const el = container.firstElementChild as HTMLElement;
    expect(el.getAttribute('data-testid')).toBe('loading-skeleton');
  });

  it('renders as a div', () => {
    const { container } = render(<Skeleton />);
    expect(container.firstElementChild?.tagName).toBe('DIV');
  });
});
