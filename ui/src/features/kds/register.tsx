import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import KdsScreen from './KdsScreen';

export function registerKdsFeature() {
  registerPage({ route: 'kds', component: KdsScreen, label: 'KDS', feature: 'kitchen-display' });
  registerNavItem({
    route: 'kds',
    label: 'KDS',
    feature: 'kitchen-display',
    i18nKey: 'nav-kds',
    section: 'operations',
    icon: icon('M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2M9 5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2M9 5h6'),
  });
}
