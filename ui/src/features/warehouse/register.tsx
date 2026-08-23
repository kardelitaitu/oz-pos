import { lazy } from 'react';
import { registerPage } from '@/platform/ui/page-registry';

const WarehouseScreen = lazy(() => import('./WarehouseScreen'));

export function registerWarehouseFeature() {
  registerPage({
    route: 'warehouse',
    component: WarehouseScreen,
    label: 'Warehouse',
    requiredRole: 'manager',
    requiredPermission: 'inventory:view',
  });
}
