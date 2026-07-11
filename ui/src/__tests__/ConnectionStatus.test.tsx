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
});
