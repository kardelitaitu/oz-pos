// ui/src/locales/test-utils.tsx — Shared Fluent wrapper for tests.
//
// Import the domain .ftl files your test needs and use `withFluent()` to
// render a component with those strings available.
//
// Usage:
//   import { withFluent } from '@/locales/test-utils';
//   import salesFtl from '@/locales/sales.ftl?raw';
//   render(withFluent(<MyComponent />, salesFtl));

import { FluentBundle, FluentResource } from '@fluent/bundle';
import { LocalizationProvider, ReactLocalization } from '@fluent/react';
import type { ReactNode, ReactElement } from 'react';

/** Wrap `children` with a Fluent provider that contains the given .ftl content. */
export function withFluent(children: ReactNode, ...ftlContents: string[]): ReactElement {
  const bundle = new FluentBundle('en-US');
  bundle.addResource(new FluentResource(ftlContents.join('\n')));
  const l10n = new ReactLocalization([bundle]);
  return <LocalizationProvider l10n={l10n}>{children}</LocalizationProvider>;
}
