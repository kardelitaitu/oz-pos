/**
 * Shared Paddle.js (v2) integration for the marketing site. Loads the v2
 * SDK lazily, initializes it with the build-time client token, and opens
 * the checkout overlay. Used by both the pricing-page button and the
 * account dashboard's subscribe section.
 *
 * v2 API: Paddle.Initialize({ token, environment }) then
 * Paddle.Checkout.open({ items, customer, customData }). NOTE: the legacy
 * URL (https://cdn.paddle.com/paddle/paddle.js) serves the v1 SDK whose
 * Setup/Checkout signatures differ — only the /v2/ URL works with this
 * code.
 *
 * Checkout is register-first (website-plan.md §5): the pricing button
 * redirects to /login until a session exists, and customData.email is the
 * account email the webhook reads to attach the subscription to the
 * tenant (apps/license-server/paddle_webhook.go).
 */
declare global {
  interface Window {
    Paddle?: {
      Initialize: (opts: { token: string; environment: 'sandbox' | 'production' }) => void;
      Checkout: {
        open: (opts: {
          items: { priceId: string; quantity: number }[];
          customer?: { email: string };
          customData?: Record<string, string>;
        }) => void;
      };
    };
  }
}

const TOKEN = import.meta.env.PUBLIC_PADDLE_CLIENT_TOKEN as string | undefined;
const ENVIRONMENT =
  (import.meta.env.PUBLIC_PADDLE_ENVIRONMENT as string | undefined) === 'sandbox' ? 'sandbox' : 'production';
const API = import.meta.env.PUBLIC_LICENSE_API_URL as string | undefined;

/** sessionStorage keys shared with AuthForm / AccountView. */
export const SESSION_KEY = 'oz_session';
export const EMAIL_KEY = 'oz_email';

/**
 * Placeholder ids (`pri_placeholder_…`) are used in the pricing content
 * until the real Paddle prices exist. Treat them as "no checkout": a real
 * client token + a fake price id would open the overlay and fail with a
 * Paddle error instead of the graceful mailto fallback.
 */
export function isPlaceholderPriceId(priceId: string | undefined): boolean {
  return Boolean(priceId && priceId.startsWith('pri_placeholder_'));
}

/** Load the v2 SDK script once, resolving when window.Paddle is ready. */
export function loadPaddle(): Promise<void> {
  return new Promise((resolve, reject) => {
    if (window.Paddle) {
      resolve();
      return;
    }
    const existing = document.getElementById('paddle-js') as HTMLScriptElement | null;
    if (existing) {
      existing.addEventListener('load', () => resolve());
      existing.addEventListener('error', () => reject(new Error('paddle failed to load')));
      return;
    }
    const script = document.createElement('script');
    script.id = 'paddle-js';
    script.src = 'https://cdn.paddle.com/paddle/v2/paddle.js';
    script.async = true;
    script.onload = () => resolve();
    script.onerror = () => reject(new Error('paddle failed to load'));
    document.head.appendChild(script);
  });
}

/**
 * Open the sandbox/live checkout overlay for a price id, prefilled with
 * the customer's account email (customData.email is what the webhook
 * reads to attach the subscription to the tenant).
 */
export async function openPaddleCheckout(priceId: string, email: string): Promise<void> {
  if (!TOKEN) throw new Error('paddle not configured');
  await loadPaddle();
  if (!window.Paddle) throw new Error('paddle unavailable');
  window.Paddle.Initialize({ token: TOKEN, environment: ENVIRONMENT });
  window.Paddle.Checkout.open({
    items: [{ priceId, quantity: 1 }],
    customer: { email },
    customData: { email },
  });
}

/** True when a session token is present (signed in). */
export function hasSession(): boolean {
  try {
    return Boolean(window.sessionStorage.getItem(SESSION_KEY));
  } catch {
    return false; // storage unavailable (private mode) — treat as signed out
  }
}

/**
 * The signed-in user's email: cached in sessionStorage (set by AuthForm
 * after verify), else fetched from /me. Returns null when not signed in
 * or the API is unreachable.
 */
export async function getSessionEmail(): Promise<string | null> {
  try {
    const cached = window.sessionStorage.getItem(EMAIL_KEY);
    if (cached) return cached;
    const token = window.sessionStorage.getItem(SESSION_KEY);
    if (!token || !API) return null;
    const res = await fetch(`${API}/api/v1/web/me`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!res.ok) return null;
    const data = (await res.json()) as { tenant?: { email?: string } };
    const email = data.tenant?.email ?? null;
    if (email) window.sessionStorage.setItem(EMAIL_KEY, email);
    return email;
  } catch {
    return null;
  }
}
