import './GatewayStatusBadge.css';

interface GatewayStatusBadgeProps {
  gatewayName: string;
  isConfigured: boolean;
  isOnline: boolean;
}

export function GatewayStatusBadge({ gatewayName, isConfigured, isOnline }: GatewayStatusBadgeProps) {
  if (!isConfigured) return null;
  return (
    <div className="gateway-badge" role="status" aria-label={`${gatewayName} ${isOnline ? 'online' : 'offline'}`}>
      <span className={`gateway-badge__dot ${isOnline ? 'online' : 'offline'}`} />
      <span className="gateway-badge__name">{gatewayName}</span>
    </div>
  );
}
