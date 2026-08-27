// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act } from 'react';
import { createRoot, type Root } from 'react-dom/client';

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

/** Set a React-controlled input value via the native setter so React picks it up. */
function setNativeValue(el: HTMLInputElement, value: string): void {
  const nativeSetter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!;
  nativeSetter.call(el, value);
  el.dispatchEvent(new Event('input', { bubbles: true }));
}

async function renderContact(locale: string) {
  const container = document.createElement('div');
  document.body.appendChild(container);
  const root = createRoot(container);
  const { default: ContactForm } = await import('../ContactForm');
  await act(async () => {
    root.render(<ContactForm locale={locale} />);
    await new Promise((r) => setTimeout(r, 10));
  });
  return { container, root };
}

function getSubmitButton(container: HTMLElement): HTMLButtonElement {
  return container.querySelector('button[type="submit"]') as HTMLButtonElement;
}

function getInputByName(container: HTMLElement, name: string): HTMLInputElement {
  return container.querySelector(`input[name="${name}"], textarea[name="${name}"], input[type="${name}"], input[type="${name}"]`) as HTMLInputElement;
}

function getInputByPlaceholder(container: HTMLElement, placeholder: string): HTMLInputElement {
  const inputs = container.querySelectorAll('input, textarea');
  for (const input of inputs) {
    if (input.getAttribute('placeholder')?.includes(placeholder)) return input as HTMLInputElement;
  }
  throw new Error(`Input with placeholder containing "${placeholder}" not found`);
}

beforeEach(() => {
  vi.clearAllMocks();
  const env = import.meta.env as Record<string, unknown>;
  env.PUBLIC_CONTACT_ENDPOINT = 'https://contact.test/api';
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('ContactForm', () => {
  it('renders the form with name, email, and message fields', async () => {
    const { container, root } = await renderContact('en');
    try {
      expect(container.querySelector('form')).not.toBeNull();
      expect(container.querySelector('input[type="text"]')).not.toBeNull();
      expect(container.querySelector('input[type="email"]')).not.toBeNull();
      expect(container.querySelector('textarea')).not.toBeNull();
      expect(getSubmitButton(container)).not.toBeNull();
    } finally {
      act(() => root.unmount());
      container.remove();
    }
  });

  it('renders i18n labels for Indonesian locale', async () => {
    const { container, root } = await renderContact('id');
    try {
      expect(container.textContent).toContain('Kirim pesan');
      expect(container.textContent).toContain('Nama');
      expect(container.textContent).toContain('Pesan');
    } finally {
      act(() => root.unmount());
      container.remove();
    }
  });

  it('submits form data and shows success state', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true });
    vi.stubGlobal('fetch', fetchMock);
    const { container, root } = await renderContact('en');
    try {
      const nameInput = getInputByPlaceholder(container, 'Your name');
      const emailInput = getInputByPlaceholder(container, 'you@example.com');
      const messageInput = container.querySelector('textarea') as HTMLTextAreaElement;

      await act(async () => {
        setNativeValue(nameInput, 'Test User');
      });
      await act(async () => {
        setNativeValue(emailInput, 'test@example.com');
      });
      await act(async () => {
        const nativeSetter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')!.set!;
        nativeSetter.call(messageInput, 'Hello, I need help with something specific.');
        messageInput.dispatchEvent(new Event('input', { bubbles: true }));
      });

      await act(async () => {
        const form = container.querySelector('form') as HTMLFormElement;
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      });

      expect(container.textContent).toContain('Thanks! Your message is on its way');
      expect(fetchMock).toHaveBeenCalledWith('https://contact.test/api', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: 'Test User',
          email: 'test@example.com',
          message: 'Hello, I need help with something specific.',
        }),
      });
    } finally {
      act(() => root.unmount());
      container.remove();
    }
  });

  it('shows error state when fetch fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 500 }));
    const { container, root } = await renderContact('en');
    try {
      await act(async () => {
        const form = container.querySelector('form') as HTMLFormElement;
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      });

      expect(container.textContent).toContain("Couldn't send your message");
    } finally {
      act(() => root.unmount());
      container.remove();
    }
  });

  it('honeypot field triggers fake success without sending', async () => {
    const fetchMock = vi.fn();
    vi.stubGlobal('fetch', fetchMock);
    const { container, root } = await renderContact('en');
    try {
      // The honeypot input has name="website" and is visually hidden.
      const honeypot = container.querySelector('input[name="website"]') as HTMLInputElement;
      expect(honeypot).not.toBeNull();
      expect(honeypot.closest('label')?.getAttribute('aria-hidden')).toBe('true');

      await act(async () => {
        setNativeValue(honeypot, 'https://spam-bot.example');
      });

      // Fill in required fields so the form can submit.
      const nameInput = getInputByPlaceholder(container, 'Your name');
      const emailInput = getInputByPlaceholder(container, 'you@example.com');
      const messageInput = container.querySelector('textarea') as HTMLTextAreaElement;
      await act(async () => {
        setNativeValue(nameInput, 'Bot Name');
        setNativeValue(emailInput, 'bot@spam.com');
        const nativeSetter = Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value')!.set!;
        nativeSetter.call(messageInput, 'Buy cheap watches now!!!');
        messageInput.dispatchEvent(new Event('input', { bubbles: true }));
      });

      await act(async () => {
        const form = container.querySelector('form') as HTMLFormElement;
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      });

      // Shows success without calling fetch (bot was trapped).
      expect(fetchMock).not.toHaveBeenCalled();
      expect(container.textContent).toContain('Thanks! Your message is on its way');
    } finally {
      act(() => root.unmount());
      container.remove();
    }
  });

  it('"Send another message" button returns to form', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true }));
    const { container, root } = await renderContact('en');
    try {
      // Submit to reach success state (empty fields are handled by honeypot check).
      await act(async () => {
        const form = container.querySelector('form') as HTMLFormElement;
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      });
      expect(container.textContent).toContain('Thanks!');

      const sendAnother = Array.from(container.querySelectorAll('button')).find(
        (b) => b.textContent?.includes('Send another'),
      ) as HTMLButtonElement;
      expect(sendAnother).not.toBeNull();

      await act(async () => {
        sendAnother.click();
      });

      expect(container.querySelector('form')).not.toBeNull();
      expect(getSubmitButton(container)).not.toBeNull();
    } finally {
      act(() => root.unmount());
      container.remove();
    }
  });

  it('submit button is disabled while sending', async () => {
    let resolveFetch: (v: unknown) => void;
    vi.stubGlobal(
      'fetch',
      vi.fn(() => new Promise((resolve) => { resolveFetch = resolve; })),
    );
    const { container, root } = await renderContact('en');
    try {
      await act(async () => {
        const form = container.querySelector('form') as HTMLFormElement;
        form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
      });

      const btn = getSubmitButton(container);
      expect(btn.disabled).toBe(true);
      expect(btn.textContent).toContain('Sending');

      await act(async () => {
        resolveFetch!({ ok: true });
        await new Promise((r) => setTimeout(r, 10));
      });
    } finally {
      act(() => root.unmount());
      container.remove();
    }
  });
});
