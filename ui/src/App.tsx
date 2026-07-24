import AppShell from '@/frontend/shell/AppShell';
import { DevToolbar } from '@/features/design/DevToolbar';
import { registerAllFeatures } from '@/features';
import { AppProviders } from '@/contexts/AppProviders';

// ── Register all feature pages & nav items ──────────────────────────
registerAllFeatures();

/**
 * Root app component. Wraps the app shell and dev toolbar with consolidated AppProviders.
 */
export default function App() {
  return (
    <AppProviders>
      <AppShell />
      <DevToolbar />
    </AppProviders>
  );
}
