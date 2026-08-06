import React from 'react';
import ReactDOM from 'react-dom/client';
import { LocalizationProvider } from '@fluent/react';
import { createEnUsLocalization } from './locales';
import { BrandProvider } from '@/contexts/BrandContext';
import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { CurrencyProvider } from '@/contexts/CurrencyContext';
import { AuthProvider } from '@/contexts/AuthContext';
import { WorkspaceProvider } from '@/contexts/WorkspaceContext';
import { ToastProvider } from '@/frontend/shared/Toast';
import TabletAppShell from '@/frontend/shell/tablet/TabletAppShell';
import { registerAllFeatures } from '@/features';
import { installPerfProbe } from './utils/perf-metrics';
import './frontend/themes/reset.css';
import './frontend/themes/tokens.css';
import './frontend/themes/components.css';
import './frontend/themes/responsive.css';

// ── Register all UI features ─────────────────────────────────────────
registerAllFeatures();

// PERF-06: expose aggregate-only runtime metrics to automated checks.
installPerfProbe();

// ── Render ───────────────────────────────────────────────────────
const l10n = createEnUsLocalization();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <LocalizationProvider l10n={l10n}>
      {/* ThemeProvider consumes useBrand() and throws without it; the
          desktop entry gets this from AppProviders, so the tablet entry
          must provide it explicitly (TAB-04 boot blocker). */}
      <BrandProvider>
      <ThemeProvider>
        <CurrencyProvider>
          <AuthProvider>
            <ToastProvider>
              <WorkspaceProvider>
                <TabletAppShell />
              </WorkspaceProvider>
            </ToastProvider>
          </AuthProvider>
        </CurrencyProvider>
      </ThemeProvider>
      </BrandProvider>
    </LocalizationProvider>
  </React.StrictMode>,
);
