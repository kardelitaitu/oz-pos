import { Component, type ErrorInfo, type ReactNode } from 'react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization } from '@fluent/react';

// Static bundle for ErrorBoundary (class component can't use hooks).
const _ebBundle = new FluentBundle('en-US');
_ebBundle.addResource(new FluentResource('error-boundary-title = Something went wrong'));
const _ebL10n = new ReactLocalization([_ebBundle]);

interface Props {
  children: ReactNode;
}

interface State {
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  override state: State = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  override componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('[ErrorBoundary]', error, info.componentStack);
  }

  override render() {
    if (this.state.error) {
      return (
        <div style={{
          display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
          minHeight: '100dvh', padding: 32, fontFamily: 'sans-serif', color: '#ef4444',
        }}>
          <h2 style={{ margin: '0 0 8px', fontSize: 18 }}>{_ebL10n.getString('error-boundary-title')}</h2>
          <p style={{ fontSize: 13, color: '#737373', maxWidth: 500, textAlign: 'center', wordBreak: 'break-all' }}>
            {this.state.error.message}
          </p>
        </div>
      );
    }
    return this.props.children;
  }
}
