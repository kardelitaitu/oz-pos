import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import CategoryManagementScreen from './CategoryManagementScreen';

export function registerCategoriesFeature() {
  registerPage({ route: 'categories', component: CategoryManagementScreen, label: 'Categories', feature: 'categories-enabled', requiredRole: 'manager' });
  registerNavItem({
    route: 'categories',
    label: 'Categories',
    feature: 'categories-enabled',
    requiredRole: 'manager',
    i18nKey: 'nav-categories',
    section: 'products',
    icon: icon('M4 6h16M4 12h16M4 18h10'),
  });
}
