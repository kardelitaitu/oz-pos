import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { RoleIcon } from '../components/RoleIcon';

describe('RoleIcon', () => {
  it('renders owner icon (crown)', () => {
    const { container } = render(<RoleIcon role="owner" />);
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders manager icon (briefcase)', () => {
    const { container } = render(<RoleIcon role="manager" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders cashier icon (register)', () => {
    const { container } = render(<RoleIcon role="cashier" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders kitchen icon (chef hat)', () => {
    const { container } = render(<RoleIcon role="kitchen" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders kitchen icon for kds alias', () => {
    const { container } = render(<RoleIcon role="kds" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders staff icon for default/unknown role', () => {
    const { container } = render(<RoleIcon role="superadmin" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders staff icon when role is null', () => {
    const { container } = render(<RoleIcon role={null} />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('accepts custom className', () => {
    const { container } = render(<RoleIcon role="staff" className="custom-icon" />);
    const svg = container.querySelector('svg');
    expect(svg!.classList.contains('custom-icon')).toBe(true);
  });

  it('accepts custom size', () => {
    const { container } = render(<RoleIcon role="staff" size={32} />);
    const svg = container.querySelector('svg');
    expect(svg!.getAttribute('width')).toBe('32');
    expect(svg!.getAttribute('height')).toBe('32');
  });

  it('defaults size to 16', () => {
    const { container } = render(<RoleIcon role="staff" />);
    const svg = container.querySelector('svg');
    expect(svg!.getAttribute('width')).toBe('16');
    expect(svg!.getAttribute('height')).toBe('16');
  });
});
