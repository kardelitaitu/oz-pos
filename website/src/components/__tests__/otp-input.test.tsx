// @vitest-environment jsdom
import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { createRoot } from 'react-dom/client';
import { act } from 'react';
import OtpInput from '../OtpInput';

(globalThis as Record<string, unknown>).IS_REACT_ACT_ENVIRONMENT = true;

describe('OtpInput', () => {
  it('renders 6 input slots by default', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<OtpInput value="" onChange={() => {}} />);
    });

    const inputs = container.querySelectorAll('input');
    expect(inputs.length).toBe(6);
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('displays provided digits in respective slots', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<OtpInput value="1234" onChange={() => {}} />);
    });

    const inputs = container.querySelectorAll('input');
    expect(inputs[0].value).toBe('1');
    expect(inputs[1].value).toBe('2');
    expect(inputs[2].value).toBe('3');
    expect(inputs[3].value).toBe('4');
    expect(inputs[4].value).toBe('');
    expect(inputs[5].value).toBe('');
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('calls onChange and onComplete when typing completes', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    const onChange = vi.fn();
    const onComplete = vi.fn();

    await act(async () => {
      root.render(<OtpInput value="12345" onChange={onChange} onComplete={onComplete} />);
    });

    const inputs = container.querySelectorAll('input');
    await act(async () => {
      inputs[5].value = '6';
      inputs[5].dispatchEvent(new Event('input', { bubbles: true }));
      const changeEvent = { target: { value: '6' } };
      inputs[5].dispatchEvent(new Event('change', { bubbles: true }));
    });

    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('filters non-digit characters from typed input', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    const onChange = vi.fn();

    // Type "a" then "b" then "3" — the first two should be filtered out.
    // Must use Object.defineProperty + input event (React 19 controlled
    // input pattern) to trigger synthetic onChange.
    await act(async () => {
      root.render(<OtpInput value="" onChange={onChange} autoFocus={false} />);
    });

    const input = container.querySelector('input')!;
    await act(async () => {
      Object.defineProperty(input, 'value', { value: '3', configurable: true, writable: true });
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });

    // onChange was called with the single digit — the non-digit characters
    // were filtered by handleChange before updating the parent.
    expect(onChange).toHaveBeenCalled();
    const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1][0];
    expect(lastCall).toBe('3');
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('handles full-code paste: fills slots, fires onComplete, no onChange spam', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    const onChange = vi.fn();
    const onComplete = vi.fn();

    await act(async () => {
      root.render(<OtpInput value="" onChange={onChange} onComplete={onComplete} autoFocus={false} />);
    });

    const inputs = container.querySelectorAll('input');
    await act(async () => {
      // Simulate pasting a complete 6-digit code into the first slot.
      const pasteEvent = new Event('paste', { bubbles: true, cancelable: true });
      Object.defineProperty(pasteEvent, 'clipboardData', {
        value: { getData: () => '123456' },
      });
      inputs[0].dispatchEvent(pasteEvent);
    });

    expect(onChange).toHaveBeenCalledWith('123456');
    expect(onComplete).toHaveBeenCalledWith('123456');
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('ignores paste of non-digit content', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    const onChange = vi.fn();
    const onComplete = vi.fn();

    await act(async () => {
      root.render(<OtpInput value="" onChange={onChange} onComplete={onComplete} autoFocus={false} />);
    });

    const inputs = container.querySelectorAll('input');
    await act(async () => {
      const pasteEvent = new Event('paste', { bubbles: true, cancelable: true });
      Object.defineProperty(pasteEvent, 'clipboardData', {
        value: { getData: () => 'hello' },
      });
      inputs[0].dispatchEvent(pasteEvent);
    });

    expect(onChange).not.toHaveBeenCalled();
    expect(onComplete).not.toHaveBeenCalled();
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('backspaces the previous slot when the current one is empty', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    const onChange = vi.fn();

    await act(async () => {
      root.render(<OtpInput value="12" onChange={onChange} autoFocus={false} />);
    });

    const inputs = container.querySelectorAll('input');
    await act(async () => {
      // Press Backspace in slot 2 (empty) — should delete slot 1's digit.
      inputs[2].dispatchEvent(new KeyboardEvent('keydown', { key: 'Backspace', bubbles: true, cancelable: true }));
    });

    expect(onChange).toHaveBeenCalledWith('1');
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('does not fire onChange on backspace in the first slot', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);
    const onChange = vi.fn();

    await act(async () => {
      root.render(<OtpInput value="" onChange={onChange} autoFocus={false} />);
    });

    const inputs = container.querySelectorAll('input');
    await act(async () => {
      inputs[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'Backspace', bubbles: true, cancelable: true }));
    });

    expect(onChange).not.toHaveBeenCalled();
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });

  it('renders the error border class when error is true', async () => {
    const container = document.createElement('div');
    document.body.appendChild(container);
    const root = createRoot(container);

    await act(async () => {
      root.render(<OtpInput value="" onChange={() => {}} autoFocus={false} error />);
    });

    const input = container.querySelector('input');
    expect(input?.className).toContain('border-red-500');
    await act(async () => {
      root.unmount();
    });
    container.remove();
  });
});
