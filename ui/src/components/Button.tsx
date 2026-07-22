import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react';

// ── Types ──────────────────────────────────────────────────────────

export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost' | 'unstyled';
export type ButtonSize = 'sm' | 'md' | 'lg';
export type ButtonState = 'ready' | 'processing';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Visual variant. @default 'primary' */
  variant?: ButtonVariant;
  /** Size preset. @default 'md' */
  size?: ButtonSize;
  /**
   * Visual state of the button.
   * - `ready`: idle, clickable (default)
   * - `processing`: shows a spinner and disables the button
   * @default 'ready'
   */
  state?: ButtonState;
  /**
   * Show a loading spinner and disable the button.
   * @deprecated Use `state="processing"` instead.
   */
  loading?: boolean;
  /** Optional icon placed before children. */
  icon?: ReactNode;
  /** Render as an icon-only button (square, equal padding). Requires an visible label or `aria-label`. */
  iconOnly?: boolean;
  /** Remove design-system variant/size styling; keep base reset, focus ring, and disabled behaviour. */
  unstyled?: boolean;
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
 * <Button variant="ghost" state="processing">Please wait…</Button>
 * <Button variant="ghost" iconOnly aria-label="Close"><CloseIcon /></Button>
 * <Button unstyled className="my-custom-toggle" aria-label="Toggle">☰</Button>
 * ```
 */
export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'primary',
      size = 'md',
      state = 'ready',
      loading,
      icon,
      iconOnly,
      unstyled,
      children,
      className,
      disabled,
      type = 'button',
      ...rest
    },
    ref,
  ) => {
    const isProcessing = state === 'processing' || loading === true;

    const classNames = [
      unstyled ? 'btn--unstyled' : 'btn',
      !unstyled && `btn--${variant}`,
      !unstyled && `btn--${size}`,
      iconOnly && 'btn--icon-only',
      className ?? '',
    ]
      .filter((v): v is string => Boolean(v))
      .join(' ');

    return (
      <button
        ref={ref}
        type={type}
        className={classNames}
        disabled={disabled || isProcessing}
        aria-disabled={disabled || isProcessing || undefined}
        aria-busy={isProcessing || undefined}
        {...rest}
      >
        {isProcessing ? (
          <span className="btn__spinner" aria-hidden="true" />
        ) : icon ? (
          <span className="btn__icon" aria-hidden="true">
            {icon}
          </span>
        ) : null}
        {isProcessing ? <span className="sr-only">{children}</span> : children}
      </button>
    );
  },
);

Button.displayName = 'Button';
