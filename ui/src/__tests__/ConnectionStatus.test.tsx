import { render, screen, act } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import ConnectionStatus from '../components/ConnectionStatus';

describe('ConnectionStatus', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal('fetch', vi.fn());
    vi.spyOn(performance, 'now').mockReturnValue(0);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('renders checking status initially', () => {
    render(<ConnectionStatus label="Auth Server" url="http://test.com" />);
    
    expect(screen.getByText('Auth Server')).toBeInTheDocument();
    const container = screen.getByTitle('Auth Server: Checking...');
    expect(container).toBeInTheDocument();
  });

  it('handles offline status when URL is empty', () => {
    render(<ConnectionStatus label="Sync Server" url="" />);
    
    const container = screen.getByTitle('Sync Server: Offline');
    expect(container).toBeInTheDocument();
  });

  it('updates to online status with latency when fetch succeeds', async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ ok: true });
    
    let time = 0;
    vi.spyOn(performance, 'now').mockImplementation(() => {
      time += 42;
      return time; // Difference will be 42ms
    });

    render(<ConnectionStatus label="Auth Server" url="http://test.com" />);

    // Fast-forward initial check
    await act(async () => {
      vi.advanceTimersByTime(100);
      // Let promises resolve
      await Promise.resolve();
    });

    expect(screen.getByTitle('Auth Server: Online (42ms)')).toBeInTheDocument();
    expect(screen.getByText('42ms')).toBeInTheDocument();
  });

  it('updates to offline status when fetch fails', async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockRejectedValueOnce(new Error('Network error'));

    render(<ConnectionStatus label="Auth Server" url="http://test.com" />);

    await act(async () => {
      vi.advanceTimersByTime(100);
      await Promise.resolve();
    });

    expect(screen.getByTitle('Auth Server: Offline')).toBeInTheDocument();
  });

  it('reacts instantly to OS offline/online events', async () => {
    (global.fetch as ReturnType<typeof vi.fn>).mockResolvedValue({ ok: true });
    render(<ConnectionStatus label="Auth Server" url="http://test.com" />);

    // Fast-forward initial check
    await act(async () => {
      vi.advanceTimersByTime(100);
      await Promise.resolve();
    });

    expect(screen.getByTitle('Auth Server: Online (0ms)')).toBeInTheDocument();

    // Trigger offline
    vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false);
    act(() => {
      window.dispatchEvent(new Event('offline'));
    });

    expect(screen.getByTitle('Auth Server: Offline')).toBeInTheDocument();

    // Trigger online
    vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(true);
    act(() => {
      window.dispatchEvent(new Event('online'));
    });

    await act(async () => {
      // Need to advance timers because online event triggers runCheck asynchronously or instantly
      // which does a fetch.
      await Promise.resolve();
    });

    expect(screen.getByTitle('Auth Server: Online (0ms)')).toBeInTheDocument();
  });

  it('supersedes an in-flight check on a second online event (ERR-08)', async () => {
    const fetchMock = global.fetch as ReturnType<typeof vi.fn>;
    // First check hangs; second online event must abort it and start fresh.
    let resolveFirst: ((v: { ok: boolean }) => void) | null = null;
    fetchMock.mockImplementationOnce(() => new Promise((r) => { resolveFirst = r; }));
    fetchMock.mockResolvedValueOnce({ ok: true });

    render(<ConnectionStatus label="Auth Server" url="http://test.com" />);

    // First (hanging) check is in flight; trigger a second online event.
    act(() => {
      window.dispatchEvent(new Event('online'));
    });
    await act(async () => {
      await Promise.resolve();
    });

    // The second check completes → online. The first, superseded check must
    // not have been able to schedule another state flip.
    expect(screen.getByTitle('Auth Server: Online (0ms)')).toBeInTheDocument();

    // Resolve the stale first check afterwards — it must be ignored.
    await act(async () => {
      resolveFirst?.({ ok: true });
      await Promise.resolve();
    });
    // Still online (no clobber), and no crash from the stale resolve.
    expect(screen.getByTitle('Auth Server: Online (0ms)')).toBeInTheDocument();
  });

  it('aborts the in-flight request and ignores its timeout on unmount (ERR-08)', async () => {
    const fetchMock = global.fetch as ReturnType<typeof vi.fn>;
    fetchMock.mockImplementationOnce(() => new Promise(() => {})); // never resolves

    const { unmount } = render(<ConnectionStatus label="Auth Server" url="http://test.com" />);

    // Simulate a slow check exceeding the 5s timeout budget.
    await act(async () => {
      vi.advanceTimersByTime(6000);
      await Promise.resolve();
    });

    // Unmount must clean up without errors.
    expect(() => unmount()).not.toThrow();
    // fetch was aborted — no state updates after unmount.
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it('ignores a stale response that resolves after a newer offline transition (ERR-08)', async () => {
    const fetchMock = global.fetch as ReturnType<typeof vi.fn>;
    let resolveSlow: ((v: { ok: boolean }) => void) | null = null;
    // First (slow) check will succeed — but an offline event lands first.
    fetchMock.mockImplementationOnce(() => new Promise((r) => { resolveSlow = r; }));

    render(<ConnectionStatus label="Auth Server" url="http://test.com" />);

    // OS goes offline before the slow check resolves.
    vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false);
    act(() => {
      window.dispatchEvent(new Event('offline'));
    });
    expect(screen.getByTitle('Auth Server: Offline')).toBeInTheDocument();

    // The stale check resolves (or aborts) afterwards — must stay offline.
    await act(async () => {
      resolveSlow?.({ ok: true });
      await Promise.resolve();
    });
    expect(screen.getByTitle('Auth Server: Offline')).toBeInTheDocument();
  });
});
