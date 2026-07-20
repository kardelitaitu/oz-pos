import { screen, fireEvent } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { renderWithProvidersSync } from '@/__tests__/test-utils/render';
import NodeTopologyEditor from '../features/stores/NodeTopologyEditor';

// NodeTopologyEditor uses useToast — wrap in providers.
// No @fluent/react mock needed — renderWithProvidersSync handles Fluent + ToastProvider.
const renderEditor = () => renderWithProvidersSync(<NodeTopologyEditor currentTier="standard" />);

describe('NodeTopologyEditor Component', () => {
  it('renders title and default retail preset nodes', () => {
    renderEditor();

    expect(screen.getByText('Visual Store & Workspace Topology Builder')).toBeInTheDocument();
    expect(screen.getByText('Downtown Branch')).toBeInTheDocument();
    expect(screen.getByText('Retail POS #1')).toBeInTheDocument();
    expect(screen.getByText('Main Warehouse')).toBeInTheDocument();
  });

  it('renders tool rack sidebar and preset buttons', () => {
    renderEditor();

    expect(screen.getByText('+ Store Node')).toBeInTheDocument();
    expect(screen.getByText('+ Workspace Node')).toBeInTheDocument();
    expect(screen.getByText('+ Warehouse Node')).toBeInTheDocument();
    expect(screen.getByText('+ Hardware Node')).toBeInTheDocument();
    expect(screen.getByText('🧪 Test Order Simulation')).toBeInTheDocument();
  });

  it('switches to restaurant & KDS preset when clicked', () => {
    renderEditor();

    const restoBtn = screen.getByText('🍽️ Resto & KDS Preset');
    fireEvent.click(restoBtn);

    expect(screen.getByText('Grand Bistro')).toBeInTheDocument();
    expect(screen.getByText('Kitchen KDS')).toBeInTheDocument();
    expect(screen.getByText('Kitchen Thermal Printer')).toBeInTheDocument();
  });

  it('toggles simulation mode on button click', () => {
    renderEditor();

    const simBtn = screen.getByText('🧪 Test Order Simulation');
    fireEvent.click(simBtn);

    expect(screen.getByText('⏹ Stop Simulation')).toBeInTheDocument();
  });
});
