import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import GiftCardsScreen from './GiftCardsScreen';

export function registerGiftCardsFeature() {
  registerPage({ route: 'gift-cards', component: GiftCardsScreen, label: 'Gift Cards', feature: 'gift-cards', requiredRole: 'manager' });
  registerNavItem({
    route: 'gift-cards',
    label: 'Gift Cards',
    feature: 'gift-cards',
    requiredRole: 'manager',
    i18nKey: 'nav-gift-cards',
    section: 'customers',
    icon: icon('M20 12H4M12 4v16M4 8h16M8 4v16'),
  });
}
