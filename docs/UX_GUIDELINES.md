# UX Guidelines: Adaptive Rendering & Fluid Scaling

## 1. The Scaling Strategy

To support a wide range of devices—from small 1366x768 checkout laptops to massive 4K self-service kiosks—OZ-POS employs **Adaptive Fluid Scaling**.

Instead of writing dozens of CSS media queries with rigid break points (e.g., changing sizes suddenly at 1080p, 1440p, etc.), we map the entire application's sizing to a single root value, and smoothly interpolate that value based on the exact width of the user's browser window.

### How It Works

1. **Relative Units (`rem`)**: 
   All typography, padding, margins, and layout widths in OZ-POS must be defined using `rem` units (where `1rem` equals the root `html` font size). 
   
   *Rule of thumb:* Never use hardcoded `px` values for large layout containers (e.g., `width: 500px`), because they will not scale. Use `rem` (e.g., `width: 31.25rem`) instead.

2. **The Base Resolution (1920px)**:
   Our standard design baseline assumes a 1920px window width. At this width, the root font size is exactly `16px`.

3. **Fluid Calculation**:
   We use a ResizeObserver-style hook (`useAppZoom` in `ZoomContext.tsx`) to recalculate the root font size on the fly whenever the window is resized.
   
   The mathematical formula is:
   `scale = window.innerWidth / 1920`
   `font-size = 16 * scale`

4. **Clamping (CSS Locks)**:
   To prevent the UI from becoming unreadably microscopic on tiny screens or comically massive on ultrawide monitors, we apply a mathematical clamp:
   - **Minimum size:** `14px` (Ensures legibility on 1366x768 monitors).
   - **Maximum size:** `28px` (Caps the scaling for 4K monitors).

## 2. Best Practices for Developers

When building UI components for OZ-POS, adhere to these guidelines to ensure they play nicely with the Adaptive Rendering engine:

- **Do not fight the browser zoom:** Because we scale based on `window.innerWidth`, native browser zoom (`Ctrl + / -`) is intentionally preserved and supported, as zooming physically shrinks or expands the reported `innerWidth` of the document.
- **Use `rem` everywhere:** Borders (1px) are the only exception. Everything else—fonts, padding, shadows, border-radii, container widths and heights—should be built using `rem` so they scale synchronously. Never use fixed `px` for layout containers (e.g., `width: 500px`); use `rem` (e.g., `width: 31.25rem`) instead.
- **Minimum sizing (`min-width` / `min-height`):** 1366×768 is the **minimum resolution that must be 100% supported**. Interactive elements (buttons, inputs) that would become unusable below this resolution **must** set a `min-width` and/or `min-height` in `px` or `rem`. This guarantees tappability at the minimum supported scale while still scaling up on larger screens.  
  Example: a square icon button can use `min-width: 64px; min-height: 64px; aspect-ratio: 1` — at 1366×768 (root font ~11.4px) this stays a comfortable tap target, and on 4K it scales up proportionally.
- **Flexbox and Grid over Absolute Positioning:** Absolute positioning (e.g., pinning something `1.5rem` from the right edge) can cause overlapping on extremely wide or scaled screens. Always prefer robust flexbox or CSS grid layouts for positioning.
- **Support Manual Overrides:** Users can disable auto-scaling via the General Settings panel and enforce a strict `100%`, `125%`, or `150%` scale. Never assume `window.innerWidth` is the sole source of truth for the active font size.

## 3. Focus Indicator Pattern

All text inputs (and any focusable element with a visible border) **must** use `box-shadow: inset` for their focus ring, not `outline`:

```css
.element:focus {
  border-color: var(--color-accent);
  box-shadow: inset 0 0 0 1px var(--color-accent);
  outline: none;
}
```

This draws the focus ring **inside** the element, sitting on top of the border. Using `outline` places the ring outside the border, which:

- Breaks the visual boundary of the input
- Requires negative `outline-offset` hacks to pull inward
- Inconsistently renders across OS/browser focus engines

Always pair with `outline: none` to suppress the native focus ring.
