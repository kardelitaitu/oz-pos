import React from 'react';
import ReactDOM from 'react-dom/client';
import { LocalizationProvider } from '@fluent/react';
import { createEnUsLocalization } from './locales';
import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { CurrencyProvider } from '@/contexts/CurrencyContext';
import { AuthProvider } from '@/contexts/AuthContext';
import { WorkspaceProvider } from '@/contexts/WorkspaceContext';
import { ToastProvider } from '@/frontend/shared/Toast';
import TabletAppShell from '@/frontend/shell/tablet/TabletAppShell';
import { registerAllFeatures } from '@/features';
import './frontend/themes/reset.css';
import './frontend/themes/tokens.css';
import './frontend/themes/components.css';
import './frontend/themes/responsive.css';

// ── Register all UI features ─────────────────────────────────────────
registerAllFeatures();

// ── Render ───────────────────────────────────────────────────────
const l10n = createEnUsLocalization();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <LocalizationProvider l10n={l10n}>
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
    </LocalizationProvider>
  </React.StrictMode>,
);
