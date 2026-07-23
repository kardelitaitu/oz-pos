import { Localized } from '@fluent/react';
import { Card } from '@/components/Card';
import ErrorBoundary from '@/components/ErrorBoundary';
import { useSettings } from '@/contexts/SettingsContext';
import type { WorkspaceCardProps } from './types';

/**
 * Read-only card displaying store-level information: name, address,
 * branch, and currency. Used in the topology inspector when a store
 * node is selected.
 */
export function StoreInfoCard({ variant = 'full-page' }: WorkspaceCardProps) {
  const { settings } = useSettings();
  const isCompact = variant === 'inspector-drawer';

  return (
    <ErrorBoundary>
      <Card
        shadow="sm"
        header={
          <h2 className="settings-section-title">
            <Localized id="workspace-store-info-heading">Store Info</Localized>
          </h2>
        }
      >
        <div className="settings-form">
          {/* Store name */}
          <div className="settings-field settings-field--horizontal">
            <span className="settings-label">
              <Localized id="workspace-store-info-name">Name</Localized>
            </span>
            <span className="settings-field-value">
              {settings.store.name || '—'}
            </span>
          </div>

          {/* Address */}
          <div className="settings-field settings-field--horizontal">
            <span className="settings-label">
              <Localized id="workspace-store-info-address">Address</Localized>
            </span>
            <span className="settings-field-value">
              {settings.store.address || '—'}
            </span>
          </div>

          {/* Branch */}
          <div className="settings-field settings-field--horizontal">
            <span className="settings-label">
              <Localized id="workspace-store-info-branch">Branch</Localized>
            </span>
            <span className="settings-field-value">
              {settings.store.branch || '—'}
            </span>
          </div>

          {/* Currency */}
          {!isCompact && (
            <div className="settings-field settings-field--horizontal">
              <span className="settings-label">
                <Localized id="workspace-store-info-currency">Currency</Localized>
              </span>
              <span className="settings-field-value">
                {settings.store.currency || '—'}
              </span>
            </div>
          )}

          {/* Tax ID */}
          {!isCompact && (
            <div className="settings-field settings-field--horizontal">
              <span className="settings-label">
                <Localized id="workspace-store-info-tax-id">Tax ID</Localized>
              </span>
              <span className="settings-field-value">
                {settings.store.taxId || '—'}
              </span>
            </div>
          )}
        </div>
      </Card>
    </ErrorBoundary>
  );
}
