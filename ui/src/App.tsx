import { lazy } from 'react';
import AppShell from '@/frontend/shell/AppShell';
import { registerAllFeatures } from '@/features';
import { AppProviders } from '@/contexts/AppProviders';

// ── Register all feature pages & nav items ──────────────────────────
registerAllFeatures();

const DevToolbar = import.meta.env.DEV
  ? lazy(() => import('@/features/design/DevToolbar').then((m) => ({ default: m.DevToolbar })))
  : null;

/**
 * Root app component. Wraps the app shell with consolidated AppProviders.
 * DevToolbar renders only in development mode.
 */
export default function App() {
  return (
    <AppProviders>
      <AppShell />
      {import.meta.env.DEV && DevToolbar && <DevToolbar />}
    </AppProviders>
  );
}
