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
});
