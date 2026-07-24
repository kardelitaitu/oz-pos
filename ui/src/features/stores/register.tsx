import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import { MultiStoreDashboardScreen } from './index';

export function registerStoresFeature() {
  registerPage({ route: 'stores', component: MultiStoreDashboardScreen, label: 'Stores', feature: 'multi-store', requiredRole: 'manager' });
  registerNavItem({
    route: 'stores',
    label: 'Stores',
    feature: 'multi-store',
    requiredRole: 'manager',
    i18nKey: 'nav-stores',
    section: 'management',
    icon: icon('M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z', <polyline points="9 22 9 12 15 12 15 22" />),
  });
}
