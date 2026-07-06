import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Input } from '@/components/Input';

describe('Input', () => {
  // ── Basic rendering ───────────────────────────────────────────

  it('renders an input element', () => {
    render(<Input />);
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  it('renders label when provided', () => {
    render(<Input label="Username" />);
    expect(screen.getByText('Username')).toBeInTheDocument();
  });

  it('associates label with input via htmlFor', () => {
    render(<Input label="Email" />);
    const label = screen.getByText('Email');
    const input = screen.getByRole('textbox');
    expect(label.getAttribute('for')).toBe(input.id);
  });

  // ── Placeholder ───────────────────────────────────────────────

  it('passes placeholder to the input', () => {
    render(<Input placeholder="Enter name" />);
    expect(screen.getByPlaceholderText('Enter name')).toBeInTheDocument();
  });

  // ── Helper text ───────────────────────────────────────────────

  it('renders helper text when provided', () => {
    render(<Input helperText="Must be unique" />);
    expect(screen.getByText('Must be unique')).toBeInTheDocument();
  });

  it('connects helper text via aria-describedby', () => {
    render(<Input helperText="Helper message" />);
    const input = screen.getByRole('textbox');
    const helperId = input.getAttribute('aria-describedby');
    expect(helperId).toBeTruthy();
    expect(document.getElementById(helperId!)).toHaveTextContent('Helper message');
  });

  // ── Error state ───────────────────────────────────────────────

  it('renders error message when provided', () => {
    render(<Input error="Invalid value" />);
    expect(screen.getByText('Invalid value')).toBeInTheDocument();
  });

  it('has aria-invalid when error is set', () => {
    render(<Input error="Required" />);
    const input = screen.getByRole('textbox');
    expect(input).toHaveAttribute('aria-invalid', 'true');
  });

  it('connects error via aria-describedby (replaces helper)', () => {
    render(<Input helperText="Hidden helper" error="Error message" />);
    const input = screen.getByRole('textbox');
    const describedBy = input.getAttribute('aria-describedby');
    expect(describedBy).toBeTruthy();
    // Helper text should NOT be visible when error is shown.
    expect(screen.queryByText('Hidden helper')).not.toBeInTheDocument();
  });

  it('has role="alert" on the error element', () => {
    render(<Input error="Something went wrong" />);
    const errorEl = screen.getByText('Something went wrong');
    expect(errorEl).toHaveAttribute('role', 'alert');
  });

  it('hides helper text when error is present', () => {
    render(<Input helperText="Help" error="Error" />);
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.queryByText('Help')).not.toBeInTheDocument();
  });

  // ── Type prop ─────────────────────────────────────────────────

  it('passes type attribute to the input', () => {
    render(<Input type="email" />);
    expect(screen.getByRole('textbox')).toHaveAttribute('type', 'email');
  });

  it('renders type="password" input', () => {
    render(<Input type="password" />);
    const input = screen.getByDisplayValue('');
    expect(input).toHaveAttribute('type', 'password');
  });

  // ── Forwarded ref ─────────────────────────────────────────────

  it('forwards ref to the input element', () => {
    const ref = { current: null as HTMLInputElement | null };
    render(<Input ref={ref} />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });

  it('focuses input via ref', () => {
    const ref = { current: null as HTMLInputElement | null };
    render(<Input ref={ref} />);
    ref.current?.focus();
    expect(document.activeElement).toBe(ref.current);
  });

  // ── User interaction ──────────────────────────────────────────

  it('allows typing into the input', async () => {
    render(<Input />);
    const input = screen.getByRole('textbox');
    await userEvent.type(input, 'Hello');
    expect(input).toHaveValue('Hello');
  });

  it('calls onChange handler', async () => {
    const onChange = vi.fn();
    render(<Input onChange={onChange} />);
    await userEvent.type(screen.getByRole('textbox'), 'a');
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  // ── Disabled state ────────────────────────────────────────────

  it('renders disabled input', () => {
    render(<Input disabled />);
    expect(screen.getByRole('textbox')).toBeDisabled();
  });

  // ── Custom className ──────────────────────────────────────────

  it('applies custom className', () => {
    const { container } = render(<Input className="custom-input" />);
    expect(container.querySelector('.custom-input')).toBeInTheDocument();
  });

  // ── Additional props ──────────────────────────────────────────

  it('passes additional HTML attributes', () => {
    render(<Input data-testid="my-input" maxLength={10} />);
    const input = screen.getByTestId('my-input');
    expect(input).toHaveAttribute('maxlength', '10');
  });
});
