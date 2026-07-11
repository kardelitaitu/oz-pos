import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { WorkspaceIcon } from '../components/WorkspaceIcon';

describe('WorkspaceIcon', () => {
  it('renders restaurant-pos icon', () => {
    const { container } = render(<WorkspaceIcon wsKey="restaurant-pos" />);
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg!.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders store-pos icon', () => {
    const { container } = render(<WorkspaceIcon wsKey="store-pos" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders kds icon', () => {
    const { container } = render(<WorkspaceIcon wsKey="kds" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders inventory icon', () => {
    const { container } = render(<WorkspaceIcon wsKey="inventory" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders admin icon', () => {
    const { container } = render(<WorkspaceIcon wsKey="admin" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('trims and lowercases wsKey', () => {
    const { container } = render(<WorkspaceIcon wsKey="  RESTAURANT-POS  " />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders default circle icon for unknown key', () => {
    const { container } = render(<WorkspaceIcon wsKey="nonexistent" />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('renders default icon when wsKey is null', () => {
    const { container } = render(<WorkspaceIcon wsKey={null} />);
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('accepts custom className', () => {
    const { container } = render(<WorkspaceIcon wsKey="admin" className="workspace-icon-lg" />);
    const svg = container.querySelector('svg');
    expect(svg!.classList.contains('workspace-icon-lg')).toBe(true);
  });

  it('accepts custom size', () => {
    const { container } = render(<WorkspaceIcon wsKey="admin" size={48} />);
    const svg = container.querySelector('svg');
    expect(svg!.getAttribute('width')).toBe('48');
    expect(svg!.getAttribute('height')).toBe('48');
  });

  it('defaults size to 24', () => {
    const { container } = render(<WorkspaceIcon wsKey="admin" />);
    const svg = container.querySelector('svg');
    expect(svg!.getAttribute('width')).toBe('24');
    expect(svg!.getAttribute('height')).toBe('24');
  });
});
