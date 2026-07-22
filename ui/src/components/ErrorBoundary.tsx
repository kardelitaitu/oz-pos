import { Component, type ErrorInfo, type ReactNode } from 'react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization } from '@fluent/react';

// Static bundle for ErrorBoundary (class component can't use hooks).
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
}

interface State {
  error: Error | null;
}

/**
 * React class-based error boundary that catches render errors and
 * displays a fallback UI with the error message and a "Try Again"
 * button that resets the error state, remounting the children.
 *
 * Uses a static Fluent bundle for localisation since class components
 * cannot use hooks.
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
        <div
          role="alert"
          style={{
            display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
            minHeight: '100dvh', padding: 32, fontFamily: 'sans-serif', color: '#ef4444',
          }}
        >
          <h2 style={{ margin: '0 0 8px', fontSize: 18 }}>
            {_ebL10n.getString('error-boundary-title')}
          </h2>
          <p
            style={{
              fontSize: 13, color: '#737373', maxWidth: 500,
              textAlign: 'center', wordBreak: 'break-all',
            }}
          >
            {this.state.error.message}
          </p>
          <button
            type="button"
            onClick={this.handleReset}
            style={{
              marginTop: 20, padding: '10px 24px', fontSize: 14,
              borderRadius: 8, border: '1px solid #ef4444', background: 'transparent',
              color: '#ef4444', cursor: 'pointer',
            }}
          >
            {_ebL10n.getString('error-boundary-retry')}
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
