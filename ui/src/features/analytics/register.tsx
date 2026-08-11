import { lazy } from 'react';
import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
const AnalyticsScreen = lazy(() => import('./AnalyticsScreen'));

export function registerAnalyticsFeature() {
  // analytics:view — owner/admin/manager only (0046 registry + 0048 scope).
  registerPage({ route: 'analytics', component: AnalyticsScreen, label: 'Staff Analytics', requiredRole: 'management' });
  registerNavItem({
    route: 'analytics',
    label: 'Staff Analytics',
    requiredRole: 'management',
    i18nKey: 'nav-analytics',
    section: 'reports',
    icon: icon('M3 3v18h18', <path d="M7 15l4-6 3 3 5-8" />),
  });
}
