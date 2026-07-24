import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { BrandProvider } from '@/contexts/BrandContext';
import { AuthProvider } from '@/contexts/AuthContext';
import { ToastProvider } from '@/frontend/shared/Toast';
import { LocaleProvider } from './i18n/LocaleContext';
import { ZoomProvider } from '@/contexts/ZoomContext';
import { HardwareAccelProvider } from '@/contexts/HardwareAccelContext';
import AppShell from '@/frontend/shell/AppShell';
import ErrorBoundary from '@/components/ErrorBoundary';
import { CurrencyProvider } from '@/contexts/CurrencyContext';
import { WorkspaceProvider } from '@/contexts/WorkspaceContext';
import { DevToolbar } from '@/features/design/DevToolbar';
import { registerAllFeatures } from '@/features';

// ── Register all feature pages & nav items ──────────────────────────
registerAllFeatures();

/**
 * Root app component. Provides theme, auth, and toast contexts,
 * then delegates to AppShell which handles routing and layout.
 */
export default function App() {
  return (
    <ErrorBoundary>
      <LocaleProvider>
        <BrandProvider>
          <ZoomProvider>
            <HardwareAccelProvider>
              <ThemeProvider>
                <CurrencyProvider>
                  <AuthProvider>
                    <ToastProvider>
                      <WorkspaceProvider>
                        <AppShell />
                        <DevToolbar />
                      </WorkspaceProvider>
                    </ToastProvider>
                  </AuthProvider>
                </CurrencyProvider>
              </ThemeProvider>
            </HardwareAccelProvider>
          </ZoomProvider>
        </BrandProvider>
      </LocaleProvider>
    </ErrorBoundary>
  );
}
