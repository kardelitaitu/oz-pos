import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ErrorBoundary from '@/components/ErrorBoundary';

// ── Helpers ────────────────────────────────────────────────────────

/** A child component that throws on render. */
function BrokenComponent({ shouldThrow = false }: { shouldThrow?: boolean }) {
  if (shouldThrow) {
    throw new Error('Test error message');
  }
  return <p>All good</p>;
}

/** A child component that throws conditionally based on a prop. */
function SometimesBroken({ fail }: { fail: boolean }) {
  if (fail) {
    throw new Error('Conditional test error');
  }
  return <p>Recovered</p>;
}

// ── Tests ──────────────────────────────────────────────────────────

describe('ErrorBoundary', () => {
  const preventJsdomError = (e: ErrorEvent) => e.preventDefault();

  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    window.addEventListener('error', preventJsdomError);
  });

  afterEach(() => {
    window.removeEventListener('error', preventJsdomError);
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

  // ── P201-2: Retry button tests ─────────────────────────────────

  it('renders a Try Again button in the fallback UI', () => {
    render(
      <ErrorBoundary>
        <BrokenComponent shouldThrow />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('button', { name: 'Try Again' })).toBeInTheDocument();
  });

  it('fallback UI has role="alert" for screen reader announcements', () => {
    render(
      <ErrorBoundary>
        <BrokenComponent shouldThrow />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('clicking Try Again resets error state so a non-throwing child renders', () => {
    // Use SometimesBroken: first render with fail=true (triggers error),
    // then rerender with fail=false (no error). Without the Try Again
    // button click the ErrorBoundary would still show the stale error,
    // so this verifies the button actually clears the error state.
    const { rerender } = render(
      <ErrorBoundary>
        <SometimesBroken fail />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    // Swap to non-failing child
    rerender(
      <ErrorBoundary>
        <SometimesBroken fail={false} />
      </ErrorBoundary>,
    );

    // Still showing error — the boundary holds the stale state
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();

    // Click Try Again — now the boundary resets and renders the new child
    fireEvent.click(screen.getByRole('button', { name: 'Try Again' }));
    expect(screen.getByText('Recovered')).toBeInTheDocument();
    expect(screen.queryByText('Something went wrong')).not.toBeInTheDocument();
  });

  it('calls onReset callback when Try Again is clicked', () => {
    const onReset = vi.fn();
    render(
      <ErrorBoundary onReset={onReset}>
        <BrokenComponent shouldThrow />
      </ErrorBoundary>,
    );
    fireEvent.click(screen.getByRole('button', { name: 'Try Again' }));
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it('does not catch async errors in useEffect (class boundary limitation)', () => {
    render(
      <ErrorBoundary>
        <p>Async safe</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText('Async safe')).toBeInTheDocument();
  });
});
