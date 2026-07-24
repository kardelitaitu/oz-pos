import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import ShiftManagementScreen from './ShiftManagementScreen';

export function registerShiftsFeature() {
  registerPage({ route: 'shifts', component: ShiftManagementScreen, label: 'Shifts', requiredRole: 'manager' });
  registerNavItem({
    route: 'shifts',
    label: 'Shifts',
    requiredRole: 'manager',
    i18nKey: 'nav-shifts',
    section: 'management',
    icon: icon('M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6', <line x1="12" y1="1" x2="12" y2="23" />),
  });
}
