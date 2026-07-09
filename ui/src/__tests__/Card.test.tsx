import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Card } from '@/components/Card';

describe('Card', () => {
  it('renders children in the card body', () => {
    render(<Card>Hello world</Card>);
    expect(screen.getByText('Hello world')).toBeTruthy();
  });

  it('renders a header when provided', () => {
    render(<Card header={<h2>Title</h2>}>Body</Card>);
    expect(screen.getByText('Title')).toBeTruthy();
    expect(document.querySelector('.card-header')).toBeTruthy();
  });

  it('renders a footer when provided', () => {
    render(<Card footer={<button>Action</button>}>Body</Card>);
    expect(screen.getByText('Action')).toBeTruthy();
    expect(document.querySelector('.card-footer')).toBeTruthy();
  });

  it('does not render header/footer when not provided', () => {
    render(<Card>Simple</Card>);
    expect(document.querySelector('.card-header')).toBeNull();
    expect(document.querySelector('.card-footer')).toBeNull();
  });

  it('applies default "none" shadow (no shadow class)', () => {
    render(<Card>No shadow</Card>);
    const card = document.querySelector('.card')!;
    expect(card.className).not.toContain('card--shadow');
  });

  it('applies shadow class when shadow prop is set', () => {
    render(<Card shadow="sm">Shadow</Card>);
    const card = document.querySelector('.card')!;
    expect(card.className).toContain('card--shadow-sm');
  });

  it('applies padding class for non-none padding', () => {
    render(<Card padding="lg">Padded</Card>);
    const card = document.querySelector('.card')!;
    expect(card.className).toContain('card--padding-lg');
  });

  it('merges additional className', () => {
    render(<Card className="my-card">Extra</Card>);
    const card = document.querySelector('.card')!;
    expect(card.className).toContain('my-card');
  });
});
