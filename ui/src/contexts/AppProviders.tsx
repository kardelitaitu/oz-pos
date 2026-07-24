import type { ReactNode } from 'react';
import ErrorBoundary from '@/components/ErrorBoundary';
import { LocaleProvider } from '@/i18n/LocaleContext';
import { BrandProvider } from '@/contexts/BrandContext';
import { ThemeProvider } from '@/frontend/shell/ThemeProvider';
import { CurrencyProvider } from '@/contexts/CurrencyContext';
import { AuthProvider } from '@/contexts/AuthContext';
import { ToastProvider } from '@/frontend/shared/Toast';
import { WorkspaceProvider } from '@/contexts/WorkspaceContext';
import { ZoomProvider } from '@/contexts/ZoomContext';
import { HardwareAccelProvider } from '@/contexts/HardwareAccelContext';

interface AppProvidersProps {
  children: ReactNode;
}

/**
 * Composite provider wrapper that establishes application contexts in optimal dependency order.
 * 
 * Order of nesting:
 * 1. ErrorBoundary (Catches root render errors)
 * 2. LocaleProvider (i18n string resolution)
 * 3. BrandProvider (Branding & Whitelabel settings)
 * 4. ThemeProvider (CSS custom properties, consumes useBrand)
 * 5. CurrencyProvider (Global currency state)
 * 6. AuthProvider (User session state)
 * 7. ToastProvider (Notification alerts)
 * 8. WorkspaceProvider (Store workspace context)
 * 9. ZoomProvider (Root font scaling)
 * 10. HardwareAccelProvider (CSS GPU acceleration flags)
 */
export function AppProviders({ children }: AppProvidersProps) {
  return (
    <ErrorBoundary>
      <LocaleProvider>
        <BrandProvider>
          <ThemeProvider>
            <CurrencyProvider>
              <AuthProvider>
                <ToastProvider>
                  <WorkspaceProvider>
                    <ZoomProvider>
                      <HardwareAccelProvider>
                        {children}
                      </HardwareAccelProvider>
                    </ZoomProvider>
                  </WorkspaceProvider>
                </ToastProvider>
              </AuthProvider>
            </CurrencyProvider>
          </ThemeProvider>
        </BrandProvider>
      </LocaleProvider>
    </ErrorBoundary>
  );
}
