import { describe, expect, it } from 'vitest';
import { NODE_HEIGHT, NODE_WIDTH } from '../features/stores/nodeTopologyClamp';
import { pointUnderCards, wireUnderCardSegments } from '../features/stores/topologyWireGeometry';

/** A horizontal store→warehouse wire at port height (NODE_PORT_Y = 224). */
function horizontalWire(y = 140) {
  const x1 = 80 + NODE_WIDTH;
  const y1 = y + 224;
  const x2 = 680;
  const y2 = y + 224;
  const dx = Math.abs(x2 - x1) * 0.5;
  return { x1, y1, x2, y2, dx };
}

describe('wireUnderCardSegments', () => {
  it('returns the clipped segment when a bezier passes under a card', () => {
    // Store (80,140) → warehouse (680,140): the wire runs along y=364.
    // A middle card at (380,260) spans x∈[380,620], y∈[260,500] — the
    // wire crosses straight through it, so the overlay path must cover
    // that x-range at y=364.
    const d = wireUnderCardSegments(horizontalWire(), [{ x: 380, y: 260 }]);
    expect(d).not.toBe('');
    expect(d).toContain('364');
  });

  it('returns an empty string when the wire crosses no card', () => {
    // Same wire, but the only other card sits BELOW the wire's y-band.
    const d = wireUnderCardSegments(horizontalWire(), [{ x: 380, y: 420 }]);
    expect(d).toBe('');
  });

  it('clips an elbow polyline segment exactly to the box', () => {
    // Elbow route with a vertical jog at x=500 from y=216 to y=116 —
    // the jog drops through a card box at (380,80).
    const polyline: Array<[number, number]> = [
      [320, 216],
      [500, 216],
      [500, 116],
      [680, 116],
    ];
    const d = wireUnderCardSegments(
      { x1: 320, y1: 216, x2: 680, y2: 116, dx: 180, polyline },
      [{ x: 380, y: 80 }],
    );
    expect(d).not.toBe('');
    // The vertical segment is clipped to the box's y-range [80, 320]:
    // the overlay starts at y=116 (inside) and ends at y=216 (inside).
    expect(d).toContain('M 500 116 L 500 216');
  });

  it('returns nothing for a wire that runs along the box edge (flush, not under)', () => {
    // The card sits exactly above the wire line — the wire is never
    // under it, so nothing is overlaid (flush is not a crossing).
    const d = wireUnderCardSegments(horizontalWire(), [{ x: 380, y: 124 }]);
    expect(d).toBe('');
  });

  it('handles multiple boxes and keeps endpoint boxes out of the caller contract', () => {
    // Two unrelated cards on the path: both must contribute segments.
    const d = wireUnderCardSegments(horizontalWire(), [
      { x: 380, y: 260 },
      { x: 470, y: 260 },
    ]);
    expect(d).not.toBe('');
    // NODE_WIDTH boxes at x=380 and x=470: [380,620] and [470,710] —
    // segments start at 396.67 (first sample inside 380) and are clipped
    // per-box, so the path contains two M commands.
    expect(d.match(/M /g)?.length).toBeGreaterThanOrEqual(2);
  });

  it('respects the real card dimensions via the shared constants', () => {
    expect(NODE_WIDTH).toBe(240);
    expect(NODE_HEIGHT).toBe(240);
  });
});

describe('pointUnderCards', () => {
  it('is true for a point inside a card box (strict interior)', () => {
    expect(pointUnderCards({ x: 500, y: 364 }, [{ x: 380, y: 260 }])).toBe(true);
  });

  it('is false for a point outside all boxes', () => {
    expect(pointUnderCards({ x: 100, y: 100 }, [{ x: 380, y: 260 }])).toBe(false);
  });

  it('is false for a point exactly on a card edge (flush)', () => {
    // The card spans [380,620]×[260,500]; a point on the right edge or the
    // bottom edge is flush, not under — the same strict-interior semantic
    // as the wire segments.
    expect(pointUnderCards({ x: 620, y: 364 }, [{ x: 380, y: 260 }])).toBe(false);
    expect(pointUnderCards({ x: 500, y: 500 }, [{ x: 380, y: 260 }])).toBe(false);
  });

  it('handles multiple boxes', () => {
    expect(pointUnderCards({ x: 500, y: 364 }, [{ x: 80, y: 140 }, { x: 380, y: 260 }])).toBe(true);
  });
});
