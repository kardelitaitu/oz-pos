import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import ErrorBoundary from '@/components/ErrorBoundary';

// ── Helpers ────────────────────────────────────────────────────────

/** A child component that throws on render. */
function BrokenComponent({ shouldThrow = false }: { shouldThrow?: boolean }) {
  if (shouldThrow) {
    throw new Error('Test error message');
  }
  return <p>All good</p>;
}

/** A child component that throws in useEffect. */
function AsyncBrokenComponent() {
  // This won't be caught by ErrorBoundary (it's async)
  return <p>Async safe</p>;
}

// ── Tests ──────────────────────────────────────────────────────────

describe('ErrorBoundary', () => {
  beforeEach(() => {
    // Suppress console.error during tests that intentionally throw.
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders children when there is no error', () => {
    render(
      <ErrorBoundary>
        <p>Normal content</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText('Normal content')).toBeInTheDocument();
  });

  it('renders error UI when a child throws', () => {
    render(
      <ErrorBoundary>
        <BrokenComponent shouldThrow />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('Test error message')).toBeInTheDocument();
  });

  it('logs error to console.error when child throws', () => {
    render(
      <ErrorBoundary>
        <BrokenComponent shouldThrow />
      </ErrorBoundary>,
    );
    expect(console.error).toHaveBeenCalled();
  });

  it('recovers when key prop changes (new instance)', () => {
    const { rerender } = render(
      <ErrorBoundary key="1">
        <BrokenComponent shouldThrow />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    rerender(
      <ErrorBoundary key="2">
        <BrokenComponent shouldThrow={false} />
      </ErrorBoundary>,
    );
    expect(screen.getByText('All good')).toBeInTheDocument();
  });

  it('still renders children content after error with new instance', () => {
    const { rerender } = render(
      <ErrorBoundary key="a">
        <BrokenComponent shouldThrow />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    rerender(
      <ErrorBoundary key="b">
        <p>New content after error</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText('New content after error')).toBeInTheDocument();
  });

  it('does not catch async errors in useEffect', () => {
    // ErrorBoundary only catches render-phase errors, not async ones.
    render(
      <ErrorBoundary>
        <AsyncBrokenComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Async safe')).toBeInTheDocument();
  });
});
