import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import ProductLookupScreen from './ProductLookupScreen';
import ProductManagementScreen from './ProductManagementScreen';
import BundleManagementScreen from './BundleManagementScreen';

export function registerProductsFeature() {
  registerPage({ route: 'products', component: ProductLookupScreen, label: 'Products' });
  registerNavItem({
    route: 'products',
    label: 'Products',
    i18nKey: 'nav-products',
    section: 'products',
    icon: icon('M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z'),
  });

  registerPage({ route: 'inventory', component: ProductManagementScreen, label: 'Inventory', requiredRole: 'manager' });
  registerNavItem({
    route: 'inventory',
    label: 'Inventory',
    requiredRole: 'manager',
    i18nKey: 'nav-inventory',
    section: 'products',
    icon: icon('M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2', <rect x="8" y="2" width="8" height="4" rx="1" ry="1" />, <line x1="8" y1="12" x2="16" y2="12" />, <line x1="8" y1="16" x2="14" y2="16" />),
  });

  registerPage({ route: 'bundles', component: BundleManagementScreen, label: 'Bundles', requiredRole: 'manager' });
  registerNavItem({
    route: 'bundles',
    label: 'Bundles',
    requiredRole: 'manager',
    i18nKey: 'nav-bundles',
    section: 'products',
    icon: icon('M16 11V7a4 4 0 0 0-8 0v4M5 9h14l1 12H4L5 9z'),
  });
}
