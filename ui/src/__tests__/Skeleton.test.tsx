import { describe, it, expect } from 'vitest';
import { renderInAct } from '@/test-utils/renderInAct';
import { Skeleton } from '@/components/Skeleton';

describe('Skeleton', () => {
  it('renders a div with default classes', async () => {
    const { container } = await renderInAct(<Skeleton />);
    const el = container.firstChild as HTMLElement;
    expect(el.tagName).toBe('DIV');
    expect(el.classList.contains('skeleton')).toBe(true);
    expect(el.classList.contains('skeleton--text')).toBe(true);
  });

  it('applies variant class', async () => {
    const { container } = await renderInAct(<Skeleton variant="circle" />);
    const el = container.firstChild as HTMLElement;
    expect(el.classList.contains('skeleton--circle')).toBe(true);
  });

  it('applies block variant', async () => {
    const { container } = await renderInAct(<Skeleton variant="block" />);
    const el = container.firstChild as HTMLElement;
    expect(el.classList.contains('skeleton--block')).toBe(true);
  });

  it('sets aria-hidden="true"', async () => {
    const { container } = await renderInAct(<Skeleton />);
    expect(container.firstChild).toHaveAttribute('aria-hidden', 'true');
  });

  it('applies custom width and height as inline styles', async () => {
    const { container } = await renderInAct(<Skeleton width="200px" height="3em" />);
    const el = container.firstChild as HTMLElement;
    expect(el.style.width).toBe('200px');
    expect(el.style.height).toBe('3em');
  });

  it('applies custom className', async () => {
    const { container } = await renderInAct(<Skeleton className="my-skeleton" />);
    const el = container.firstChild as HTMLElement;
    expect(el.classList.contains('my-skeleton')).toBe(true);
  });

  it('merges style prop with width/height', async () => {
    const { container } = await renderInAct(
      <Skeleton width="100px" height="50px" style={{ opacity: 0.5 }} />,
    );
    const el = container.firstChild as HTMLElement;
    expect(el.style.width).toBe('100px');
    expect(el.style.height).toBe('50px');
    expect(el.style.opacity).toBe('0.5');
  });

  it('forwards additional HTML attributes', async () => {
    const { container } = await renderInAct(<Skeleton data-testid="loader" />);
    expect(container.firstChild).toHaveAttribute('data-testid', 'loader');
  });

  it('renders custom style without width/height', async () => {
    const { container } = await renderInAct(<Skeleton style={{ borderRadius: '50%' }} />);
    const el = container.firstChild as HTMLElement;
    expect(el.style.borderRadius).toBe('50%');
  });
});
