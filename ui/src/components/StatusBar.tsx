import { useLocalization } from '@fluent/react';
import { requiredLocalized } from '@/frontend/shared';
import { useToast } from '@/frontend/shared/Toast';
import Tooltip from '@/frontend/shell/Tooltip';
import { AUTH_SERVICE_URL } from '@/utils/service-url';
import { useHealthLatency } from '@/hooks/useHealthLatency';
import { useSyncConnection } from '@/hooks/useSyncConnection';
import { useVersionStatus } from '@/hooks/useVersionStatus';
import type { HealthLatencyInfo } from '@/hooks/useHealthLatency';
import type { VersionStatusInfo } from '@/hooks/useVersionStatus';
import './StatusBar.css';

// ── Latency color thresholds (ms) ─────────────────────────────────
const LATENCY_GOOD_MAX = 999; // green
const LATENCY_WARN_MAX = 2999; // yellow
// >= 3000 or unreachable → red

type DotTone = 'good' | 'warn' | 'bad' | 'checking';

/** Map a health latency result to a color tone. */
function latencyTone(latencyMs: number | null, state: HealthLatencyInfo['state']): DotTone {
  if (state === 'checking') return 'checking';
  if (state === 'offline' || latencyMs === null) return 'bad';
  if (latencyMs <= LATENCY_GOOD_MAX) return 'good';
  if (latencyMs <= LATENCY_WARN_MAX) return 'warn';
  return 'bad';
}

/** Icon glyphs (Lucide-style, 24x24 stroke icons, uniform style). */
function KeyIcon() {
  return (
    <svg className="statusbar-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4" />
    </svg>
  );
}

function SyncIcon() {
  return (
    <svg className="statusbar-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <polyline points="23 4 23 10 17 10" />
      <polyline points="1 20 1 14 7 14" />
      <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
    </svg>
  );
}

function DownloadIcon() {
  return (
    <svg className="statusbar-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </svg>
  );
}

const ICONS = {
  key: KeyIcon,
  sync: SyncIcon,
  download: DownloadIcon,
} as const;

interface StatusItemProps {
  kind: keyof typeof ICONS;
  tone: DotTone;
  label: string;
  tooltip: string;
  onClick?: () => void;
}

/** One colored icon button with a hover tooltip + click toast. */
function StatusItem({ kind, tone, label, tooltip, onClick }: StatusItemProps) {
  const Icon = ICONS[kind];
  return (
    <Tooltip content={tooltip} position="top" showDelay={300} portal nowrap>
      <button
        type="button"
        className={`statusbar-item statusbar-tone--${tone}`}
        aria-label={label}
        aria-describedby={undefined}
        onClick={onClick}
      >
        <Icon />
      </button>
    </Tooltip>
  );
}

/**
 * Single status area with three colored SVG icons — auth, sync, version.
 *
 * Colors follow latency thresholds: green < 1 s, yellow 1–3 s, red >= 3 s
 * (or unreachable). While checking, the icon slowly blinks grey. Hovering
 * shows a native tooltip; clicking raises a toast with the same detail.
 *
 * When `bare` is set the surrounding pill/box chrome is omitted so the icon
 * row can sit inline inside a larger container (e.g. the shell StatusBar).
 */
export default function StatusBar({ bare = false }: { bare?: boolean }) {
  const { l10n } = useLocalization();
  const { addToast } = useToast();

  const auth = useHealthLatency(AUTH_SERVICE_URL);
  const sync = useSyncConnection();
  const version = useVersionStatus();

  // ── Labels (localized) ─────────────────────────────────────────
  const authLabel = requiredLocalized(l10n, 'staff-login-connection-auth');
  const syncLabel = requiredLocalized(l10n, 'staff-login-connection-sync');
  const versionLabel = requiredLocalized(l10n, 'statusbar-version-label');

  // ── Auth item ──────────────────────────────────────────────────
  const authTone = latencyTone(auth.latencyMs, auth.state);
  const authTooltip = healthTooltip(l10n, auth, authLabel);

  // ── Sync item (useSyncConnection: checking/connected/disconnected) ─
  const syncTone: DotTone =
    sync.state === 'checking' ? 'checking' : sync.state === 'connected' ? latencyTone(sync.latencyMs, 'online') : 'bad';
  const syncTooltip =
    sync.state === 'checking'
      ? requiredLocalized(l10n, 'statusbar-checking-msg', { name: syncLabel })
      : sync.state === 'disconnected'
        ? requiredLocalized(l10n, 'statusbar-offline-msg', { name: syncLabel })
        : requiredLocalized(l10n, 'statusbar-latency-msg', { name: syncLabel, ms: String(sync.latencyMs ?? 0) });

  // ── Version item (2 states: latest / update) ───────────────────
  const versionTone: DotTone =
    version.state === 'checking' ? 'checking' : version.state === 'update' ? 'warn' : 'good';
  const versionTooltip =
    version.state === 'checking'
      ? requiredLocalized(l10n, 'statusbar-checking-msg', { name: versionLabel })
      : version.state === 'update'
        ? requiredLocalized(l10n, 'statusbar-version-update-msg')
        : requiredLocalized(l10n, 'statusbar-version-latest-msg');

  const notify = (msg: string) => addToast({ type: 'info', message: msg });

  return (
    <div className={`statusbar${bare ? ' statusbar--bare' : ''}`} role="group" aria-label={requiredLocalized(l10n, 'statusbar-group-aria')}>
      <StatusItem kind="key" tone={authTone} label={authLabel} tooltip={authTooltip} onClick={() => notify(authTooltip)} />
      <StatusItem kind="sync" tone={syncTone} label={syncLabel} tooltip={syncTooltip} onClick={() => notify(syncTooltip)} />
      <StatusItem kind="download" tone={versionTone} label={versionLabel} tooltip={versionTooltip} onClick={() => notify(versionTooltip)} />
    </div>
  );
}

/** Localize a health item's tooltip from its state + latency. */
function healthTooltip(
  l10n: ReturnType<typeof useLocalization>['l10n'],
  info: HealthLatencyInfo,
  name: string,
): string {
  if (info.state === 'checking') return requiredLocalized(l10n, 'statusbar-checking-msg', { name });
  if (info.state === 'offline' || info.latencyMs === null) {
    return requiredLocalized(l10n, 'statusbar-offline-msg', { name });
  }
  return requiredLocalized(l10n, 'statusbar-latency-msg', { name, ms: String(info.latencyMs) });
}

// Re-export type for tests.
export type { VersionStatusInfo };