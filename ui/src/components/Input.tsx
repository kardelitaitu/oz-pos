import { forwardRef, useId, type InputHTMLAttributes } from 'react';

// ── Types ──────────────────────────────────────────────────────────

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  /** Visible label rendered above the input. */
  label?: string;
  /** Helper text shown below the input when there is no error. */
  helperText?: string;
  /** Error message that replaces helper text and marks the input invalid. */
  error?: string;
}

// ── Component ─────────────────────────────────────────────────────

/**
 * Accessible text input with label, error state, and helper text.
 *
 * Examples:
 * ```tsx
 * <Input label="Name" placeholder="Enter your name" />
 * <Input label="Email" type="email" error="Invalid email" />
 * <Input label="SKU" helperText="Must be unique" />
 * ```
 */
export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ label, helperText, error, className, id: externalId, ...rest }, ref) => {
    const autoId = useId();
    const inputId = externalId ?? autoId;
    const helperId = `${inputId}-helper`;
    const errorId = `${inputId}-error`;

    const classNames = ['input-wrapper', className ?? '']
      .filter(Boolean)
      .join(' ');

    const describedBy = error ? errorId : helperText ? helperId : undefined;

    return (
      <div className={classNames}>
        {label && (
          <label className="input-label" htmlFor={inputId}>
            {label}
          </label>
        )}

        <input
          ref={ref}
          id={inputId}
          className="input-field"
          aria-invalid={error ? true : undefined}
          aria-describedby={describedBy}
          {...rest}
        />

        {error && (
          <span id={errorId} className="input-error" role="alert">
            {error}
          </span>
        )}

        {!error && helperText && (
          <span id={helperId} className="input-helper">
            {helperText}
          </span>
        )}
      </div>
    );
  },
);

Input.displayName = 'Input';
