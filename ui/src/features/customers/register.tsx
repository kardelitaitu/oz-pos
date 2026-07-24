import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import CustomerManagementScreen from './CustomerManagementScreen';

export function registerCustomersFeature() {
  registerPage({ route: 'customers', component: CustomerManagementScreen, label: 'Customers' });
  registerNavItem({
    route: 'customers',
    label: 'Customers',
    i18nKey: 'nav-customers',
    section: 'customers',
    icon: icon('M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2', <circle cx="9" cy="7" r="4" />),
  });
}
