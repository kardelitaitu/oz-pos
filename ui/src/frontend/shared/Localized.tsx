// Re-export the `<Localized>` component from `@fluent/react` so that
// components only import from `@/components/Localized`. The wrapper
// is a one-liner today, but it gives us a place to add defaults (e.g.
// `data-testid` propagation) without rewriting every JSX site.

export { Localized } from '@fluent/react';
