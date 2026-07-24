import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import OfflineQueueScreen from './OfflineQueueScreen';

export function registerOfflineFeature() {
  registerPage({ route: 'offline-queue', component: OfflineQueueScreen, label: 'Offline Queue', requiredRole: 'manager' });
  registerNavItem({
    route: 'offline-queue',
    label: 'Offline Queue',
    requiredRole: 'manager',
    i18nKey: 'nav-offline-queue',
    section: 'management',
    icon: icon('M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z'),
  });
}
