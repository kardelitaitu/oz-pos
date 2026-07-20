import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import NodeTopologyEditor from '../features/stores/NodeTopologyEditor';

// Mock fluent localization
vi.mock('@fluent/react', () => ({
  Localized: ({ children }: { children: React.ReactNode }) => children,
  useLocalization: () => ({
    l10n: {
      getString: (id: string) => id,
    },
  }),
}));

describe('NodeTopologyEditor Component', () => {
  it('renders title and default retail preset nodes', () => {
    render(<NodeTopologyEditor currentTier="standard" />);

    expect(screen.getByText('Visual Store & Workspace Topology Builder')).toBeInTheDocument();
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    expect(screen.getByText('Retail POS #1')).toBeInTheDocument();
    expect(screen.getByText('Main Warehouse')).toBeInTheDocument();
  });

  it('renders tool rack sidebar and preset buttons', () => {
    render(<NodeTopologyEditor currentTier="standard" />);

    expect(screen.getByText('+ Store Node')).toBeInTheDocument();
    expect(screen.getByText('+ Workspace Node')).toBeInTheDocument();
    expect(screen.getByText('+ Warehouse Node')).toBeInTheDocument();
    expect(screen.getByText('+ Hardware Node')).toBeInTheDocument();
    expect(screen.getByText('🧪 Test Order Simulation')).toBeInTheDocument();
  });

  it('switches to restaurant & KDS preset when clicked', () => {
    render(<NodeTopologyEditor currentTier="standard" />);

    const restoBtn = screen.getByText('🍽️ Resto & KDS Preset');
    fireEvent.click(restoBtn);

    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
    expect(screen.getByText('Kitchen KDS')).toBeInTheDocument();
    expect(screen.getByText('Kitchen Thermal Printer')).toBeInTheDocument();
  });

  it('toggles simulation mode on button click', () => {
    render(<NodeTopologyEditor currentTier="standard" />);

    const simBtn = screen.getByText('🧪 Test Order Simulation');
    fireEvent.click(simBtn);

    expect(screen.getByText('⏹ Stop Simulation')).toBeInTheDocument();
  });
});
