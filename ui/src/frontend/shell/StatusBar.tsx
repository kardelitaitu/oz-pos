// ── StatusBar ──────────────────────────────────────────────────────
// Full-width bar at the bottom of the app layout, inspired by VS Code's
// status bar. Shows connection state, version, license, gateway status,
// workspace switcher, user switcher (ADR #6), and theme toggle.
// ────────────────────────────────────────────────────────────────────

import { useState, useCallback } from "react";
import { Localized, useLocalization } from "@fluent/react";
import { useGatewayStatus } from "@/hooks/useGatewayStatus";
import { useWorkspaceNav } from "@/hooks/useWorkspaceNav";
import { useAuth } from "@/contexts/AuthContext";
import ThemeToggle from "./ThemeToggle";
import Tooltip from "./Tooltip";
import FastPINOverlay from "@/components/FastPINOverlay";
import "./StatusBar.css";

/**
 * Thin status bar spanning the full width at the bottom of the app.
 *
 * Left segment:
 *   • Connection dot + version label
 *   • Gateway status pill (Stripe)
 *
 * Right segment:
 *   • Switch Workspace button
 *   • Theme Toggle (sun/moon icon)
 */
export default function StatusBar() {
  const { l10n } = useLocalization();
  const stripeStatus = useGatewayStatus();
  const { goToWorkspacePicker } = useWorkspaceNav();
  const { session } = useAuth();
  const [showFastPIN, setShowFastPIN] = useState(false);

  const handleOpenFastPIN = useCallback(() => setShowFastPIN(true), []);
  const handleCloseFastPIN = useCallback(() => setShowFastPIN(false), []);

  const connectionLabel = stripeStatus.online
    ? l10n.getString("status-bar-connected")
    : l10n.getString("status-bar-disconnected");
  const connectionDotClass = stripeStatus.online
    ? "statusbar-dot--online"
    : "statusbar-dot--offline";

  return (
    <>
      {/* ADR #6: Fast user switching overlay */}
      <FastPINOverlay open={showFastPIN} onClose={handleCloseFastPIN} />

      <footer
        className="app-statusbar"
        role="status"
        aria-label="Application status"
      >
        {/* ── Left segment: connection + version ── */}
        <div className="statusbar-left">
          <Tooltip content={connectionLabel} position="top">
            <div className="statusbar-segment">
              <span
                className={`statusbar-dot ${connectionDotClass}`}
                aria-hidden="true"
              />
              <span className="statusbar-version">
                OZ-POS Enterprise v0.0.10
              </span>
            </div>
          </Tooltip>

          {/* Gateway status pill */}
          {stripeStatus.configured && (
            <Tooltip
              content={
                stripeStatus.online
                  ? l10n.getString("gateway-status-online-aria", {
                      name: "Stripe",
                    })
                  : l10n.getString("gateway-status-offline-aria", {
                      name: "Stripe",
                    })
              }
              position="top"
            >
              <div className="statusbar-segment statusbar-gateway">
                <span
                  className={`statusbar-dot ${stripeStatus.online ? "statusbar-dot--online" : "statusbar-dot--offline"}`}
                  aria-hidden="true"
                />
                <span className="statusbar-gateway-name">Stripe</span>
              </div>
            </Tooltip>
          )}

          <span className="statusbar-divider" aria-hidden="true" />
          <span className="statusbar-license">Proprietary License</span>
        </div>

        {/* ── Right segment: user + workspace + theme ── */}
        <div className="statusbar-right">
          {/* ADR #6: Fast user switching for shared touchscreens */}
          {session && (
            <>
              <Tooltip
                content={l10n.getString("fastpin-switch-user")}
                position="top"
              >
                <button
                  type="button"
                  className="statusbar-btn"
                  onClick={handleOpenFastPIN}
                  aria-label={l10n.getString("fastpin-switch-user")}
                >
                  <svg
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    width="14"
                    height="14"
                    aria-hidden="true"
                  >
                    <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
                    <circle cx="12" cy="7" r="4" />
                    <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
                    <path d="M16 3.13a4 4 0 0 1 0 7.75" />
                  </svg>
                  <Localized id="fastpin-switch-user">
                    <span>Switch User</span>
                  </Localized>
                </button>
              </Tooltip>
              <span className="statusbar-divider" aria-hidden="true" />
            </>
          )}

          <Tooltip
            content={l10n.getString("nav-switch-workspace")}
            position="top"
          >
            <button
              type="button"
              className="statusbar-btn"
              onClick={goToWorkspacePicker}
              aria-label={l10n.getString("nav-switch-workspace")}
            >
              <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
                width="14"
                height="14"
                aria-hidden="true"
              >
                <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
                <line x1="8" y1="21" x2="16" y2="21" />
                <line x1="12" y1="17" x2="12" y2="21" />
              </svg>
              <Localized id="nav-switch-workspace">
                <span>Workspace</span>
              </Localized>
            </button>
          </Tooltip>

          <span className="statusbar-divider" aria-hidden="true" />

          <ThemeToggle />
        </div>
      </footer>
    </>
  );
}
