import { Component, type ErrorInfo, type ReactNode } from 'react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization } from '@fluent/react';
import './ErrorBoundary.css';

// Static bundle for ErrorBoundary (class component can't use hooks).
// This is the *emergency* fallback used only when no localized strings are
// injected from a wrapper (e.g. when LocaleProvider itself failed to mount).
// Prefer `LocalizedErrorBoundary` inside the locale tree so users see copy
// in the active language. Keys live in shared.ftl / shared.id.ftl.
const _ebBundle = new FluentBundle('en-US');
_ebBundle.addResource(new FluentResource(`
error-boundary-title = Something went wrong
error-boundary-retry = Try Again
`));
const _ebL10n = new ReactLocalization([_ebBundle]);

interface Props {
  children: ReactNode;
  /** Called after the user clicks "Try Again" — useful for external side effects (e.g. reload data). */
  onReset?: () => void;
  /** Pre-localized fallback title (injected by LocalizedErrorBoundary). */
  title?: string;
  /** Pre-localized retry label (injected by LocalizedErrorBoundary). */
  retryLabel?: string;
}

interface State {
  error: Error | null;
}

/**
 * React class-based error boundary that catches render errors and
 * displays a fallback UI with the error message and a "Try Again"
 * button that resets the error state, remounting the children.
 *
 * ERR-02: layout and colors come from token-backed CSS classes
 * (ErrorBoundary.css) instead of inline styles, so the fallback follows
 * the active brand theme, dark mode, and forced-colors overrides.
 *
 * Because class components cannot use hooks, localization is injected via
 * the optional `title`/`retryLabel` props (see `LocalizedErrorBoundary`).
 * The module-level English bundle is a locale-independent emergency
 * fallback for the case where localization itself is unavailable.
 */
export default class ErrorBoundary extends Component<Props, State> {
  override state: State = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  override componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('[ErrorBoundary]', error, info.componentStack);
  }

  private handleReset = () => {
    this.setState({ error: null });
    this.props.onReset?.();
  };

  override render() {
    if (this.state.error) {
      return (
        <div role="alert" className="error-boundary">
          <div className="error-boundary__card">
            <h2 className="error-boundary__title">
              {this.props.title ?? _ebL10n.getString('error-boundary-title')}
            </h2>
            <p className="error-boundary__message">{this.state.error.message}</p>
            <button
              type="button"
              className="error-boundary__retry"
              onClick={this.handleReset}
            >
              {this.props.retryLabel ?? _ebL10n.getString('error-boundary-retry')}
            </button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
