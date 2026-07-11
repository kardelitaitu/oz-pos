import { useLocalization } from '@fluent/react';
import './GatewayStatusBadge.css';

interface GatewayStatusBadgeProps {
  gatewayName: string;
  isConfigured: boolean;
  isOnline: boolean;
}

export function GatewayStatusBadge({ gatewayName, isConfigured, isOnline }: GatewayStatusBadgeProps) {
  const { l10n } = useLocalization();
  if (!isConfigured) return null;
  return (
    <div className="gateway-badge" role="status" aria-label={l10n.getString(isOnline ? 'gateway-status-online-aria' : 'gateway-status-offline-aria', { name: gatewayName })}>
      <span className={`gateway-badge__dot ${isOnline ? 'online' : 'offline'}`} />
      <span className="gateway-badge__name">{gatewayName}</span>
    </div>
  );
}
