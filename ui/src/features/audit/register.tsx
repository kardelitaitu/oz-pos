import { registerPage } from '@/platform/ui/page-registry';
import { registerNavItem } from '@/platform/ui/menu-registry';
import { icon } from '@/platform/ui/icon';
import AuditLogScreen from './AuditLogScreen';

export function registerAuditFeature() {
  registerPage({ route: 'audit-log', component: AuditLogScreen, label: 'Audit Log', requiredRole: 'manager' });
  registerNavItem({
    route: 'audit-log',
    label: 'Audit Log',
    requiredRole: 'manager',
    i18nKey: 'nav-audit-log',
    section: 'management',
    icon: icon('M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z', <polyline points="14 2 14 8 20 8" />, <line x1="16" y1="13" x2="8" y2="13" />, <line x1="16" y1="17" x2="8" y2="17" />),
  });
}
