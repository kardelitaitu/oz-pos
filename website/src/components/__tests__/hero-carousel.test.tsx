// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act } from 'react';
import { createRoot } from 'react-dom/client';
import HeroCarousel from '../HeroCarousel';
import type { SlideId } from '../HeroCarousel';

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

const LABELS: Record<SlideId, string> = {
  restaurant: 'Restaurant',
  retail: 'Retail',
  kitchen: 'Kitchen',
  warehouse: 'Warehouse',
  topology: 'Topology',
};
const DESCRIPTIONS: Record<SlideId, string> = {
  restaurant: 'Order, pay, and manage tables',
  retail: 'Ring up sales',
  kitchen: 'KDS order queue',
  warehouse: 'Stock, transfers, and purchase orders',
  topology: 'Visual editor',
};
const COMING_SOON = 'Screenshot coming soon';

describe('HeroCarousel', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    sessionStorage.clear();
    localStorage.clear();
    // jsdom lacks matchMedia — default to "no reduced motion".
    vi.stubGlobal('matchMedia', () => ({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  async function renderCarousel() {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    await act(async () => {
      root.render(<HeroCarousel labels={LABELS} descriptions={DESCRIPTIONS} comingSoon={COMING_SOON} />);
    });
    return {
      container,
      unmount: async () => {
        await act(async () => {
          root.unmount();
        });
        container.remove();
      },
    };
  }

  function pillButtons(container: HTMLElement): HTMLButtonElement[] {
    return Array.from(container.querySelectorAll('button'));
  }

  it('renders five pill buttons with the localized labels', async () => {
    const { container, unmount } = await renderCarousel();
    const buttons = pillButtons(container);
    expect(buttons).toHaveLength(5);
    for (const label of Object.values(LABELS)) {
      expect(container.textContent).toContain(label);
    }
    await unmount();
  });

  it('starts on the first slide and marks it current', async () => {
    const { container, unmount } = await renderCarousel();
    const buttons = pillButtons(container);
    expect(buttons[0].getAttribute('aria-current')).toBe('true');
    expect(buttons[1].getAttribute('aria-current')).toBeNull();
    // Track sits at translateX(-0%) with a transform transition
    const track = container.querySelector('[role="group"] > div') as HTMLElement;
    expect(track.style.transform).toMatch(/translateX\(-?0%\)/);
    expect(track.style.transition).toMatch(/transform 700ms/);
    await unmount();
  });

  it('auto-advances every dwell period and wraps from last back to first', async () => {
    const { container, unmount } = await renderCarousel();
    const buttons = pillButtons(container);

    // After 10 s the carousel should be on slide 2.
    await act(async () => {
      vi.advanceTimersByTime(10000);
    });
    expect(buttons[1].getAttribute('aria-current')).toBe('true');

    // Walk to the last slide (3 more advances → index 4), then wrap to the
    // first (1 more advance → index 0).
    await act(async () => {
      vi.advanceTimersByTime(3 * 10000);
    });
    expect(buttons[4].getAttribute('aria-current')).toBe('true');
    await act(async () => {
      vi.advanceTimersByTime(10000);
    });
    expect(buttons[0].getAttribute('aria-current')).toBe('true');
    await unmount();
  });

  it('clicking a pill jumps straight to that slide', async () => {
    const { container, unmount } = await renderCarousel();
    const buttons = pillButtons(container);

    await act(async () => {
      buttons[3].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    expect(buttons[3].getAttribute('aria-current')).toBe('true');
    const track = container.querySelector('[role="group"] > div') as HTMLElement;
    expect(track.style.transform).toBe('translateX(-300%)');

    // The manual jump must reset the auto-advance timer: advancing less
    // than one dwell after a click must not move the slide again.
    await act(async () => {
      vi.advanceTimersByTime(9999);
    });
    expect(buttons[3].getAttribute('aria-current')).toBe('true');
    await unmount();
  });

  it('clicking the same pill still resets the auto timer', async () => {
    const { container, unmount } = await renderCarousel();
    const buttons = pillButtons(container);

    // Advance to slide 2 (index 1).
    await act(async () => {
      vi.advanceTimersByTime(10000);
    });
    expect(buttons[1].getAttribute('aria-current')).toBe('true');

    // Clicking the current pill restarts the dwell countdown.
    await act(async () => {
      buttons[1].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    await act(async () => {
      vi.advanceTimersByTime(9999);
    });
    expect(buttons[1].getAttribute('aria-current')).toBe('true');
    await unmount();
  });

  it('pauses auto-advance only when hovering the pill bar, not the stage', async () => {
    const { container, unmount } = await renderCarousel();
    const buttons = pillButtons(container);
    const stage = container.querySelector('[role="group"]') as HTMLElement;
    // The pill bar is the flex sibling directly after the stage.
    const pillBar = stage.nextElementSibling as HTMLElement;

    // Hovering the STAGE (the big mockup) must NOT pause auto-slide —
    // a resting cursor on the hero would otherwise stall it forever.
    await act(async () => {
      stage.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
    });
    await act(async () => {
      vi.advanceTimersByTime(10000);
    });
    expect(buttons[1].getAttribute('aria-current')).toBe('true');

    // Hovering the PILL BAR does pause it.
    await act(async () => {
      pillBar.dispatchEvent(new MouseEvent('mouseover', { bubbles: true }));
    });
    await act(async () => {
      vi.advanceTimersByTime(3 * 10000);
    });
    expect(buttons[1].getAttribute('aria-current')).toBe('true');

    // Leaving the pill bar resumes.
    await act(async () => {
      pillBar.dispatchEvent(new MouseEvent('mouseout', { bubbles: true, relatedTarget: null }));
    });
    await act(async () => {
      vi.advanceTimersByTime(10000);
    });
    expect(buttons[2].getAttribute('aria-current')).toBe('true');
    await unmount();
  });

  it('respects prefers-reduced-motion: no auto-advance', async () => {
    vi.stubGlobal('matchMedia', () => ({ matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() }));
    const { container, unmount } = await renderCarousel();
    const buttons = pillButtons(container);

    await act(async () => {
      vi.advanceTimersByTime(10 * 10000);
    });
    // Never auto-advanced.
    expect(buttons[0].getAttribute('aria-current')).toBe('true');

    // Manual click still works.
    await act(async () => {
      buttons[2].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    });
    expect(buttons[2].getAttribute('aria-current')).toBe('true');
    await unmount();
  });

  it('renders all slides with their distinct labels and descriptions', async () => {
    const { container, unmount } = await renderCarousel();
    expect(container.textContent).toContain('Restaurant');
    expect(container.textContent).toContain('Retail');
    expect(container.textContent).toContain('Kitchen');
    expect(container.textContent).toContain('Warehouse');
    expect(container.textContent).toContain('Topology');
    expect(container.textContent).toContain('Stock, transfers, and purchase orders');
    await unmount();
  });
});
