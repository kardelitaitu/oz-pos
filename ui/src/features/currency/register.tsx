import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import ExchangeRateScreen from './ExchangeRateScreen';

export function registerCurrencyFeature() {
  registerPage({ route: 'exchange-rates', component: ExchangeRateScreen, label: 'Exchange Rates', requiredRole: 'manager' });
  registerNavItem({
    route: 'exchange-rates',
    label: 'Exchange Rates',
    requiredRole: 'manager',
    i18nKey: 'nav-exchange-rates',
    section: 'finance',
    icon: icon('M12 1v22M17 5H9.5a3.5 3.5 0 0 0 0 7h5a3.5 3.5 0 0 1 0 7H6', <line x1="12" y1="1" x2="12" y2="23" />),
  });
}
