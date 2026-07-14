/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useContext,
  useState,
  useCallback,
  useMemo,
  useEffect,
  type ReactNode,
} from "react";
import { staffLogin, type LoginSessionDto } from "@/api/staff";

const SESSION_KEY = "oz-pos-session";
const SESSION_MAX_AGE_MS = 24 * 60 * 60 * 1000; // 24 hours

/** Restore persisted session on mount (survives F5 / Vite HMR).
 *  Clears sessions older than 24 hours to prevent stale tokens. */
function loadSession(): LoginSessionDto | null {
  try {
    const raw = localStorage.getItem(SESSION_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (
      parsed &&
      typeof parsed.display_name === "string" &&
      typeof parsed.role_name === "string" &&
      typeof parsed._storedAt === "number"
    ) {
      // Discard sessions older than the max age
      if (Date.now() - parsed._storedAt > SESSION_MAX_AGE_MS) {
        localStorage.removeItem(SESSION_KEY);
        return null;
      }
      const { _storedAt, ...session } = parsed;
      return session as LoginSessionDto;
    }
  } catch { /* malformed — ignore */ }
  return null;
}

function persistSession(session: LoginSessionDto | null) {
  try {
    if (session) {
      localStorage.setItem(
        SESSION_KEY,
        JSON.stringify({ ...session, _storedAt: Date.now() }),
      );
    } else {
      localStorage.removeItem(SESSION_KEY);
    }
  } catch { /* quota exceeded — ignore */ }
}

// ── Types ───────────────────────────────────────────────────────────

export interface AuthState {
  /** The currently logged-in user's session, or null if not logged in. */
  session: LoginSessionDto | null;
  /** Whether a login attempt is in progress. */
  loading: boolean;
  /** Error message from the last failed login attempt. */
  error: string | null;
}

export interface AuthContextValue extends AuthState {
  /** Attempt to log in with username and PIN. */
  login: (username: string, pin: string) => Promise<void>;
  /** Log out the current user. */
  logout: () => void;
  /** Clear any login error. */
  clearError: () => void;
  /** Whether the current user has manager-level access or higher. */
  isManager: boolean;
  /** Whether the current user has owner-level access. */
  isOwner: boolean;
  /**
   * ADR #6: Hot-swap the session to a different user without triggering
   * the full login/logout lifecycle (no workspace reset). Used by
   * FastPINOverlay for shared touchscreen operator switching.
   */
  swapSession: (session: LoginSessionDto) => void;
}

// ── Context ─────────────────────────────────────────────────────────

const AuthContext = createContext<AuthContextValue | null>(null);

// ── Provider ────────────────────────────────────────────────────────

interface AuthProviderProps {
  children: ReactNode;
  /** Called when the user successfully logs in. Can be used to dismiss the login screen. */
  onLogin?: () => void;
}

/**
 * Provides authentication state and login/logout actions to the app.
 *
 * Wrap this around the app shell. Before the user logs in, show the
 * StaffLoginScreen. After login, the session is available via `useAuth()`.
 */
export function AuthProvider({ children, onLogin }: AuthProviderProps) {
  const [session, setSession] = useState<LoginSessionDto | null>(() => loadSession());
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Persist session changes to localStorage
  useEffect(() => {
    persistSession(session);
  }, [session]);

  const login = useCallback(
    async (username: string, pin: string) => {
      setLoading(true);
      setError(null);
      try {
        const result = await staffLogin({ username, pin });
        setSession(result.session);
        onLogin?.();
      } catch (err) {
        const message = err instanceof Error ? err.message : "Login failed";
        setError(message);
      } finally {
        setLoading(false);
      }
    },
    [onLogin],
  );

  const logout = useCallback(() => {
    setSession(null);
    setError(null);
  }, []);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  /**
   * ADR #6: Replace the current session with a new user's session
   * without triggering the login flow (no loading/error reset, no onLogin).
   * This is the hot-swap path used by FastPINOverlay.
   */
  const swapSession = useCallback((newSession: LoginSessionDto) => {
    setSession(newSession);
    setError(null);
  }, []);

  const isManager =
    session?.role_name === "manager" ||
    session?.role_name === "owner" ||
    session?.role_name === "admin" ||
    session?.role_name === "role-manager" ||
    session?.role_name === "role-owner" ||
    session?.role_name === "role-admin";
  const isOwner =
    session?.role_name === "owner" ||
    session?.role_name === "admin" ||
    session?.role_name === "manager" ||
    session?.role_name === "role-owner" ||
    session?.role_name === "role-admin" ||
    session?.role_name === "role-manager";

  const value = useMemo<AuthContextValue>(
    () => ({
      session,
      loading,
      error,
      login,
      logout,
      clearError,
      swapSession,
      isManager,
      isOwner,
    }),
    [
      session,
      loading,
      error,
      login,
      logout,
      clearError,
      swapSession,
      isManager,
      isOwner,
    ],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

// ── Hook ────────────────────────────────────────────────────────────

/**
 * Access the current authentication state and login/logout actions.
 *
 * @example
 * ```tsx
 * const { session, login, logout, isManager } = useAuth();
 * if (!session) return <StaffLoginScreen />;
 * ```
 */
export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth must be used within an <AuthProvider>");
  }
  return ctx;
}
