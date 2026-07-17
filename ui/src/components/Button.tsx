import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react';

// ── Types ──────────────────────────────────────────────────────────

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost';
export type ButtonSize = 'sm' | 'md' | 'lg';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual variant. @default 'primary' */
  variant?: ButtonVariant;
  /** Size preset. @default 'md' */
  size?: ButtonSize;
  /** Show a loading spinner and disable the button. */
  loading?: boolean;
  /** Optional icon placed before children. */
  icon?: ReactNode;
  /** Button text or content. */
  children: ReactNode;
}

// ── Component ─────────────────────────────────────────────────────

/**
 * Reusable button using design tokens.
 *
 * Examples:
 * ```tsx
 * <Button>Save</Button>
 * <Button variant="danger" size="lg">Delete</Button>
 * <Button variant="ghost" loading>Please wait…</Button>
 * ```
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'primary',
      size = 'md',
      loading = false,
      icon,
      children,
      className,
      disabled,
      type = 'button',
      ...rest
    },
    ref,
  ) => {
    const classNames = [
      'btn',
      `btn--${variant}`,
      `btn--${size}`,
      className ?? '',
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <button
        ref={ref}
        type={type}
        className={classNames}
        disabled={disabled || loading}
        aria-disabled={disabled || loading || undefined}
        aria-busy={loading || undefined}
        {...rest}
      >
        {loading ? (
          <span className="btn__spinner" aria-hidden="true" />
        ) : icon ? (
          <span className="btn__icon" aria-hidden="true">
            {icon}
          </span>
        ) : null}
        <span className={loading ? 'sr-only' : undefined}>{children}</span>
      </button>
    );
  },
);

Button.displayName = 'Button';
