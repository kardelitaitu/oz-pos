import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { FluentBundle, FluentResource } from '@fluent/bundle';
import { ReactLocalization, LocalizationProvider } from '@fluent/react';
import { Spinner } from '@/components/Spinner';

const bundle = new FluentBundle('en-US');
bundle.addResource(new FluentResource('spinner-label = Loading'));
const l10n = new ReactLocalization([bundle]);

function renderSpinner(props: Parameters<typeof Spinner>[0]) {
  return render(
    <LocalizationProvider l10n={l10n}>
      <Spinner {...props} />
    </LocalizationProvider>,
  );
}

describe('Spinner', () => {
  it('renders a span with role="status"', () => {
    renderSpinner({});
    const el = screen.getByRole('status');
    expect(el.tagName).toBe('SPAN');
  });

  it('has default aria-label from Fluent', () => {
    renderSpinner({});
    expect(screen.getByRole('status')).toHaveAttribute('aria-label', 'Loading');
  });

  it('uses custom label as aria-label when provided', () => {
    renderSpinner({ label: 'Saving...' });
    expect(screen.getByRole('status')).toHaveAttribute('aria-label', 'Saving...');
  });

  it('renders label text inside the spinner', () => {
    renderSpinner({ label: 'Please wait' });
    expect(screen.getByText('Please wait')).toBeInTheDocument();
  });

  it('does not render label span when no label given', () => {
    const { container } = renderSpinner({});
    expect(container.querySelector('.spinner__label')).toBeNull();
  });

  it('applies default size (md) class', () => {
    renderSpinner({});
    const el = screen.getByRole('status');
    expect(el.classList.contains('spinner--md')).toBe(true);
  });

  it('applies sm size class', () => {
    renderSpinner({ size: 'sm' });
    expect(screen.getByRole('status')).toHaveClass('spinner--sm');
  });

  it('applies lg size class', () => {
    renderSpinner({ size: 'lg' });
    expect(screen.getByRole('status')).toHaveClass('spinner--lg');
  });

  it('applies custom className', () => {
    renderSpinner({ className: 'my-spinner' });
    expect(screen.getByRole('status')).toHaveClass('my-spinner');
  });

  it('forwards additional HTML attributes', () => {
    renderSpinner({ 'aria-label': 'loader' });
    expect(screen.getByRole('status')).toBeInTheDocument();
  });
});
