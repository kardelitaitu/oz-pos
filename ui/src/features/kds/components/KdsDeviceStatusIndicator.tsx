import { memo, useState, useEffect, useCallback } from 'react';
import { requiredLocalized } from '@/frontend/shared';
import { useLocalization } from '@fluent/react';
import {
  listKdsDevicesScoped,
  type KdsDevice,
  type KdsConnectionStatus,
} from '@/api/kds';
import './KdsDeviceStatusIndicator.css';

/** Props for the KdsDeviceStatusIndicator. */
export interface KdsDeviceStatusIndicatorProps {
  /** Session token for scoped API calls. */
  sessionToken: string;
  /** How often to poll for device status (ms). Defaults to 10000. */
  pollIntervalMs?: number;
}

/** Map connection status to a display label and CSS modifier. */
const STATUS_DISPLAY: Record<
  KdsConnectionStatus,
  { labelKey: string; className: string }
> = {
  connected: {
    labelKey: 'kds-device-status-connected',
    className: 'kds-device-status--connected',
  },
  disconnected: {
    labelKey: 'kds-device-status-disconnected',
    className: 'kds-device-status--disconnected',
  },
  stale: {
    labelKey: 'kds-device-status-stale',
    className: 'kds-device-status--stale',
  },
};

/**
 * KdsDeviceStatusIndicator — compact badge showing the connection
 * status of all KDS devices for the current Restaurant POS.
 *
 * Displays a small pill with a colored dot and device count, expandable
 * to show individual device names and their statuses.
 */
export const KdsDeviceStatusIndicator = memo(
  function KdsDeviceStatusIndicator({
    sessionToken,
    pollIntervalMs = 10_000,
  }: KdsDeviceStatusIndicatorProps) {
    const { l10n } = useLocalization();
    const [devices, setDevices] = useState<KdsDevice[]>([]);
    const [expanded, setExpanded] = useState(false);
    const [loading, setLoading] = useState(false);

    const fetchDevices = useCallback(async () => {
      if (!sessionToken) return;
      setLoading(true);
      try {
      const result = await listKdsDevicesScoped(sessionToken);
      setDevices(Array.isArray(result) ? result : []);
      } catch {
        // Silent — indicator is non-critical UI.
      } finally {
        setLoading(false);
      }
    }, [sessionToken]);

    // Poll for device status.
    useEffect(() => {
      fetchDevices();
      const interval = setInterval(fetchDevices, pollIntervalMs);
      return () => clearInterval(interval);
    }, [fetchDevices, pollIntervalMs]);

    // Don't render if no devices are registered.
    if ((!devices || devices.length === 0) && !loading) {
      return null;
    }

    const connectedCount = devices.filter(
      (d) => d.connection_status === 'connected',
    ).length;
    const totalCount = devices.length;

    // Determine overall status: all connected > some connected > none.
    const overallStatus: KdsConnectionStatus =
      connectedCount === totalCount
        ? 'connected'
        : connectedCount > 0
          ? 'stale'
          : 'disconnected';

    const statusDisplay = STATUS_DISPLAY[overallStatus];

    return (
      <div className="kds-device-status-container">
        <button
          className={`kds-device-status ${statusDisplay.className}`}
          onClick={() => setExpanded((p) => !p)}
          aria-expanded={expanded}
          aria-label={requiredLocalized(l10n, 'kds-device-status-aria', {
            connected: String(connectedCount),
            total: String(totalCount),
          })}
        >
          <span className="kds-device-status-dot" aria-hidden="true" />
          <span className="kds-device-status-count">
            {connectedCount}/{totalCount}
          </span>
        </button>

        {expanded && (
          <div
            className="kds-device-status-dropdown"
            role="list"
            aria-label={requiredLocalized(l10n, 'kds-device-list-aria')}
          >
            {devices.map((device) => {
              const deviceDisplay =
                STATUS_DISPLAY[device.connection_status];
              return (
                <div
                  key={device.id}
                  className={`kds-device-status-item ${deviceDisplay.className}`}
                  role="listitem"
                >
                  <span
                    className="kds-device-status-dot kds-device-status-dot--sm"
                    aria-hidden="true"
                  />
                  <span className="kds-device-status-name">
                    {device.name}
                  </span>
                  <span className="kds-device-status-label">
                    {requiredLocalized(l10n, deviceDisplay.labelKey)}
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </div>
    );
  },
);
