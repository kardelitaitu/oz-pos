import { useState, useMemo, useRef, useEffect } from 'react';
import { Localized } from '@fluent/react';
import { Button } from '@/components/Button';
import './NodeTopologyEditor.css';

// ── Types ──────────────────────────────────────────────────────────

export type NodeType = 'store' | 'workspace' | 'warehouse' | 'hardware';
export type WireDirection = 'one-way' | 'two-way';

export interface TopologyNodeData {
  id: string;
  type: NodeType;
  name: string;
  subtitle?: string;
  x: number;
  y: number;
  tierRequirement?: 'pro' | 'enterprise';
  telemetryBadge?: string;
  telemetryStatus?: 'online' | 'warning' | 'offline';
  metadata?: Record<string, string>;
}

export interface TopologyWireData {
  id: string;
  fromNodeId: string;
  toNodeId: string;
  direction: WireDirection;
  label?: string; // e.g. "Priority 1", "Priority 2 (Fallback)"
}

export interface NodeTopologyEditorProps {
  currentTier?: 'free' | 'one_time' | 'standard' | 'pro' | 'enterprise';
  onSave?: (nodes: TopologyNodeData[], wires: TopologyWireData[]) => void;
}

// ── Presets ────────────────────────────────────────────────────────

const PRESET_RETAIL: { nodes: TopologyNodeData[]; wires: TopologyWireData[] } = {
  nodes: [
    { id: 'store-1', type: 'store', name: 'Downtown Branch', subtitle: 'Primary Store', x: 80, y: 140, telemetryBadge: 'Online (2 POS)', telemetryStatus: 'online' },
    { id: 'ws-1', type: 'workspace', name: 'Retail POS #1', subtitle: 'Main Checkout', x: 340, y: 80, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'wh-1', type: 'warehouse', name: 'Main Warehouse', subtitle: 'Primary Storage', x: 600, y: 140, telemetryBadge: '1,250 items', telemetryStatus: 'online' },
  ],
  wires: [
    { id: 'w-1', fromNodeId: 'store-1', toNodeId: 'ws-1', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-2', fromNodeId: 'ws-1', toNodeId: 'wh-1', direction: 'one-way', label: 'Stock Deduct (P1)' },
  ],
};

const PRESET_RESTAURANT: { nodes: TopologyNodeData[]; wires: TopologyWireData[] } = {
  nodes: [
    { id: 'store-1', type: 'store', name: 'Grand Bistro', subtitle: 'Main Branch', x: 80, y: 180, telemetryBadge: 'Online (3 Terminals)', telemetryStatus: 'online' },
    { id: 'ws-1', type: 'workspace', name: 'Resto POS #1', subtitle: 'Dining Room', x: 340, y: 80, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'ws-kds', type: 'workspace', name: 'Kitchen KDS', subtitle: 'Line Cook Display', x: 340, y: 260, telemetryBadge: 'Active', telemetryStatus: 'online' },
    { id: 'wh-kitchen', type: 'warehouse', name: 'Kitchen Pantry', subtitle: 'Cold & Dry Storage', x: 600, y: 180, telemetryBadge: '⚠️ 12 Low Stock', telemetryStatus: 'warning' },
    { id: 'hw-prn', type: 'hardware', name: 'Kitchen Thermal Printer', subtitle: 'LAN 192.168.1.100', x: 600, y: 320, telemetryBadge: 'Ready', telemetryStatus: 'online' },
  ],
  wires: [
    { id: 'w-1', fromNodeId: 'store-1', toNodeId: 'ws-1', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-2', fromNodeId: 'store-1', toNodeId: 'ws-kds', direction: 'one-way', label: 'Binds Store' },
    { id: 'w-3', fromNodeId: 'ws-1', toNodeId: 'wh-kitchen', direction: 'one-way', label: 'Stock Deduct' },
    { id: 'w-4', fromNodeId: 'ws-kds', toNodeId: 'hw-prn', direction: 'one-way', label: 'Ticket Print' },
  ],
};

export default function NodeTopologyEditor({
  currentTier = 'standard',
  onSave,
}: NodeTopologyEditorProps) {
  const [nodes, setNodes] = useState<TopologyNodeData[]>(PRESET_RETAIL.nodes);
  const [wires, setWires] = useState<TopologyWireData[]>(PRESET_RETAIL.wires);

  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [selectedWireId, setSelectedWireId] = useState<string | null>(null);

  // Simulation mode & pulse animation state
  const [isSimulating, setIsSimulating] = useState(false);
  const [simPulseStep, setSimPulseStep] = useState(0);

  // Dragging state
  const [draggingNodeId, setDraggingNodeId] = useState<string | null>(null);
  const dragOffsetRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  // Pan & Zoom
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [isPanning, setIsPanning] = useState(false);
  const panStartRef = useRef<{ x: number; y: number }>({ x: 0, y: 0 });

  // New wire creation state
  const [connectingFromNodeId, setConnectingFromNodeId] = useState<string | null>(null);

  // Lock check based on active tier
  const isProAllowed = useMemo(() => ['pro', 'enterprise'].includes(currentTier), [currentTier]);

  // Simulation runner
  useEffect(() => {
    if (!isSimulating) return;
    const interval = setInterval(() => {
      setSimPulseStep((prev) => (prev + 1) % 100);
    }, 30);
    return () => clearInterval(interval);
  }, [isSimulating]);

  // Handle Node Mouse Down for Dragging
  const handleNodeMouseDown = (e: React.MouseEvent, nodeId: string) => {
    e.stopPropagation();
    setSelectedNodeId(nodeId);
    setSelectedWireId(null);
    setDraggingNodeId(nodeId);

    const node = nodes.find((n) => n.id === nodeId);
    if (node) {
      dragOffsetRef.current = {
        x: e.clientX / zoom - node.x,
        y: e.clientY / zoom - node.y,
      };
    }
  };

  // Handle Canvas Mouse Move (Dragging Node or Panning)
  const handleCanvasMouseMove = (e: React.MouseEvent) => {
    if (draggingNodeId) {
      const newX = Math.max(20, Math.round(e.clientX / zoom - dragOffsetRef.current.x));
      const newY = Math.max(20, Math.round(e.clientY / zoom - dragOffsetRef.current.y));

      setNodes((prev) =>
        prev.map((n) => (n.id === draggingNodeId ? { ...n, x: newX, y: newY } : n)),
      );
    } else if (isPanning) {
      setPan({
        x: e.clientX - panStartRef.current.x,
        y: e.clientY - panStartRef.current.y,
      });
    }
  };

  // Handle Mouse Up
  const handleCanvasMouseUp = () => {
    setDraggingNodeId(null);
    setIsPanning(false);
  };

  // Handle Canvas Pan Start
  const handleCanvasMouseDown = (e: React.MouseEvent) => {
    if (e.target === e.currentTarget || (e.target as HTMLElement).tagName === 'svg') {
      setSelectedNodeId(null);
      setSelectedWireId(null);
      if (e.button === 0 || e.button === 1) {
        setIsPanning(true);
        panStartRef.current = { x: e.clientX - pan.x, y: e.clientY - pan.y };
      }
    }
  };

  // Handle Zoom
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
    setZoom((prev) => Math.min(2.0, Math.max(0.4, prev * zoomFactor)));
  };

  // Add Node from Tool Rack
  const handleAddNode = (type: NodeType) => {
    if (type === 'warehouse' && !isProAllowed && nodes.filter((n) => n.type === 'warehouse').length >= 1) {
      alert('Multi-Warehouse storage locations require a Pro Tier license.');
      return;
    }

    const id = `${type}-${Date.now()}`;
    const newNode: TopologyNodeData = {
      id,
      type,
      name: `New ${type.charAt(0).toUpperCase() + type.slice(1)}`,
      subtitle: type === 'store' ? 'Branch' : type === 'workspace' ? 'Register' : type === 'warehouse' ? 'Storage' : 'Peripheral',
      x: 200 + Math.random() * 100,
      y: 150 + Math.random() * 100,
      telemetryBadge: 'Ready',
      telemetryStatus: 'online',
    };

    setNodes((prev) => [...prev, newNode]);
    setSelectedNodeId(id);
  };

  // Handle Port Click to Start/Finish Connection
  const handlePortClick = (e: React.MouseEvent, nodeId: string) => {
    e.stopPropagation();

    if (!connectingFromNodeId) {
      setConnectingFromNodeId(nodeId);
    } else if (connectingFromNodeId === nodeId) {
      setConnectingFromNodeId(null);
    } else {
      // Connect connectingFromNodeId -> nodeId
      const fromNode = nodes.find((n) => n.id === connectingFromNodeId);
      const toNode = nodes.find((n) => n.id === nodeId);

      if (fromNode && toNode) {
        // Multi-warehouse wire check for Pro license
        const existingWarehouseWires = wires.filter((w) => {
          const fn = nodes.find((n) => n.id === w.fromNodeId);
          const tn = nodes.find((n) => n.id === w.toNodeId);
          return fn?.type === 'workspace' && tn?.type === 'warehouse';
        });

        if (fromNode.type === 'workspace' && toNode.type === 'warehouse' && existingWarehouseWires.length >= 1 && !isProAllowed) {
          alert('Multi-warehouse stock deduction fallback wires require a Pro Tier license.');
          setConnectingFromNodeId(null);
          return;
        }

        const newWireId = `wire-${Date.now()}`;
        const isWarehouseWire = fromNode.type === 'workspace' && toNode.type === 'warehouse';
        const label = isWarehouseWire
          ? existingWarehouseWires.length === 0 ? 'Stock Deduct (P1)' : `Fallback (P${existingWarehouseWires.length + 1})`
          : 'Connected';

        setWires((prev) => [
          ...prev,
          {
            id: newWireId,
            fromNodeId: connectingFromNodeId,
            toNodeId: nodeId,
            direction: 'one-way',
            label,
          },
        ]);
      }
      setConnectingFromNodeId(null);
    }
  };

  // Toggle Wire Direction (1-Way <-> 2-Way)
  const handleToggleWireDirection = (wireId: string) => {
    setWires((prev) =>
      prev.map((w) => {
        if (w.id === wireId) {
          return {
            ...w,
            direction: w.direction === 'one-way' ? 'two-way' : 'one-way',
          };
        }
        return w;
      }),
    );
  };

  // Delete Selected Element
  const handleDeleteSelected = () => {
    if (selectedNodeId) {
      setNodes((prev) => prev.filter((n) => n.id !== selectedNodeId));
      setWires((prev) => prev.filter((w) => w.fromNodeId !== selectedNodeId && w.toNodeId !== selectedNodeId));
      setSelectedNodeId(null);
    } else if (selectedWireId) {
      setWires((prev) => prev.filter((w) => w.id !== selectedWireId));
      setSelectedWireId(null);
    }
  };

  // Selected Node Details
  const selectedNode = useMemo(() => nodes.find((n) => n.id === selectedNodeId), [nodes, selectedNodeId]);

  return (
    <div className="node-topology-editor">
      {/* ── Top Header Bar ────────────────────────────────────────────── */}
      <div className="node-topology-header">
        <div className="node-topology-header-title">
          <Localized id="topology-builder-title">
            <h2>Visual Store & Workspace Topology Builder</h2>
          </Localized>
          <span className={`topology-tier-badge tier-${currentTier}`}>
            {currentTier.toUpperCase()} TIER
          </span>
        </div>

        <div className="node-topology-header-actions">
          <Button
            variant={isSimulating ? 'primary' : 'secondary'}
            onClick={() => setIsSimulating(!isSimulating)}
            className="simulation-btn"
          >
            {isSimulating ? '⏹ Stop Simulation' : '🧪 Test Order Simulation'}
          </Button>

          <Button
            variant="secondary"
            onClick={() => { setNodes(PRESET_RETAIL.nodes); setWires(PRESET_RETAIL.wires); }}
          >
            🛒 Retail Preset
          </Button>

          <Button
            variant="secondary"
            onClick={() => { setNodes(PRESET_RESTAURANT.nodes); setWires(PRESET_RESTAURANT.wires); }}
          >
            🍽️ Resto & KDS Preset
          </Button>

          <Button
            variant="primary"
            onClick={() => onSave?.(nodes, wires)}
          >
            ✓ Apply Topology Changes
          </Button>
        </div>
      </div>

      <div className="node-topology-main">
        {/* ── Left Sidebar Tool Rack ──────────────────────────────────── */}
        <div className="node-tool-rack">
          <h3>Palette Tools</h3>
          <p className="tool-rack-desc">Drag or click to spawn topology nodes:</p>

          <button className="tool-card" onClick={() => handleAddNode('store')}>
            <span className="tool-card-icon">🏢</span>
            <div className="tool-card-info">
              <strong>+ Store Node</strong>
              <span>Store Branch Profile</span>
            </div>
          </button>

          <button className="tool-card" onClick={() => handleAddNode('workspace')}>
            <span className="tool-card-icon">🛒</span>
            <div className="tool-card-info">
              <strong>+ Workspace Node</strong>
              <span>POS / Register Instance</span>
            </div>
          </button>

          <button
            className={`tool-card ${!isProAllowed && nodes.some((n) => n.type === 'warehouse') ? 'locked' : ''}`}
            onClick={() => handleAddNode('warehouse')}
          >
            <span className="tool-card-icon">📦</span>
            <div className="tool-card-info">
              <strong>+ Warehouse Node</strong>
              <span>Storage Location</span>
            </div>
            {!isProAllowed && nodes.some((n) => n.type === 'warehouse') && (
              <span className="lock-badge">🔒 Pro</span>
            )}
          </button>

          <button className="tool-card" onClick={() => handleAddNode('hardware')}>
            <span className="tool-card-icon">🖨️</span>
            <div className="tool-card-info">
              <strong>+ Hardware Node</strong>
              <span>Printer / KDS Peripheral</span>
            </div>
          </button>

          <hr className="tool-rack-divider" />

          {selectedNodeId || selectedWireId ? (
            <Button variant="secondary" onClick={handleDeleteSelected} className="delete-btn">
              🗑 Delete Selected Element
            </Button>
          ) : null}

          <div className="canvas-controls-mini">
            <span>Zoom: {Math.round(zoom * 100)}%</span>
            <Button variant="secondary" onClick={() => { setZoom(1); setPan({ x: 0, y: 0 }); }}>
              Reset View
            </Button>
          </div>
        </div>

        {/* ── Interactive Node Graph Canvas ────────────────────────────── */}
        <div
          className="node-canvas-container"
          onMouseMove={handleCanvasMouseMove}
          onMouseUp={handleCanvasMouseUp}
          onMouseDown={handleCanvasMouseDown}
          onWheel={handleWheel}
        >
          <div
            className="node-canvas-viewport"
            style={{
              transform: `translate(${pan.x}px, ${pan.y}px) scale(${zoom})`,
            }}
          >
            {/* SVG Layer for Arrow Connections & Animated Simulation Pulses */}
            <svg className="node-wires-svg">
              <defs>
                {/* Arrow markers for 1-way and 2-way wires */}
                <marker
                  id="arrow-end"
                  viewBox="0 0 10 10"
                  refX="8"
                  refY="5"
                  markerWidth="7"
                  markerHeight="7"
                  orient="auto-start-reverse"
                >
                  <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--color-primary, #3b82f6)" />
                </marker>

                <marker
                  id="arrow-start"
                  viewBox="0 0 10 10"
                  refX="2"
                  refY="5"
                  markerWidth="7"
                  markerHeight="7"
                  orient="auto-start-reverse"
                >
                  <path d="M 10 0 L 0 5 L 10 10 z" fill="var(--color-primary, #3b82f6)" />
                </marker>
              </defs>

              {wires.map((wire) => {
                const fromNode = nodes.find((n) => n.id === wire.fromNodeId);
                const toNode = nodes.find((n) => n.id === wire.toNodeId);
                if (!fromNode || !toNode) return null;

                // Center positions of nodes
                const x1 = fromNode.x + 100;
                const y1 = fromNode.y + 40;
                const x2 = toNode.x + 100;
                const y2 = toNode.y + 40;

                // Control points for smooth bezier curve
                const dx = Math.abs(x2 - x1) * 0.5;
                const pathD = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;

                // Simulation pulse interpolation
                const pulseX = x1 + (x2 - x1) * (simPulseStep / 100);
                const pulseY = y1 + (y2 - y1) * (simPulseStep / 100);

                const isSelected = selectedWireId === wire.id;

                return (
                  <g key={wire.id} className={`wire-group ${isSelected ? 'wire-selected' : ''}`}>
                    {/* Background hit line */}
                    <path
                      d={pathD}
                      className="wire-hitbox"
                      onClick={(e) => {
                        e.stopPropagation();
                        setSelectedWireId(wire.id);
                        setSelectedNodeId(null);
                      }}
                    />

                    {/* Visible Wire Connection */}
                    <path
                      d={pathD}
                      className={`wire-path ${wire.direction}`}
                      markerEnd="url(#arrow-end)"
                      markerStart={wire.direction === 'two-way' ? 'url(#arrow-start)' : undefined}
                    />

                    {/* Wire Label & Direction Toggle Button */}
                    <g
                      transform={`translate(${(x1 + x2) / 2}, ${(y1 + y2) / 2})`}
                      className="wire-label-group"
                      onClick={() => handleToggleWireDirection(wire.id)}
                    >
                      <rect x="-40" y="-12" width="80" height="22" rx="11" className="wire-label-bg" />
                      <text x="0" y="3" textAnchor="middle" className="wire-label-text">
                        {wire.direction === 'two-way' ? '↔' : '→'} {wire.label || ''}
                      </text>
                    </g>

                    {/* Animated Energy Pulse during Simulation */}
                    {isSimulating && (
                      <circle
                        cx={pulseX}
                        cy={pulseY}
                        r="6"
                        className="wire-simulation-pulse"
                      />
                    )}
                  </g>
                );
              })}
            </svg>

            {/* Render Draggable Topology Nodes */}
            {nodes.map((node) => {
              const isSelected = selectedNodeId === node.id;
              const isConnectingSource = connectingFromNodeId === node.id;

              return (
                <div
                  key={node.id}
                  className={`topology-node node-type-${node.type} ${isSelected ? 'node-selected' : ''} ${isConnectingSource ? 'node-connecting-source' : ''}`}
                  style={{ left: `${node.x}px`, top: `${node.y}px` }}
                  onMouseDown={(e) => handleNodeMouseDown(e, node.id)}
                >
                  <div className="node-header">
                    <span className="node-type-icon">
                      {node.type === 'store' ? '🏢' : node.type === 'workspace' ? '🛒' : node.type === 'warehouse' ? '📦' : '🖨️'}
                    </span>
                    <span className="node-title">{node.name}</span>
                  </div>

                  <div className="node-body">
                    <span className="node-subtitle">{node.subtitle}</span>
                    {node.telemetryBadge && (
                      <span className={`node-telemetry-badge telemetry-${node.telemetryStatus || 'online'}`}>
                        {node.telemetryBadge}
                      </span>
                    )}
                  </div>

                  {/* Node Connection Port Sockets */}
                  <button
                    className={`node-port-socket ${isConnectingSource ? 'port-active' : ''}`}
                    onClick={(e) => handlePortClick(e, node.id)}
                    title="Click to connect wire to another node"
                  >
                    ●
                  </button>
                </div>
              );
            })}
          </div>
        </div>

        {/* ── Right Node Inspector Drawer ───────────────────────────── */}
        {selectedNode && (
          <div className="node-inspector-drawer">
            <div className="inspector-header">
              <h3>Node Inspector</h3>
              <Button variant="secondary" onClick={() => setSelectedNodeId(null)}>✕</Button>
            </div>

            <div className="inspector-content">
              <label className="inspector-field">
                <span>Node Name</span>
                <input
                  type="text"
                  value={selectedNode.name}
                  onChange={(e) => {
                    const name = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, name } : n)));
                  }}
                />
              </label>

              <label className="inspector-field">
                <span>Subtitle / Location</span>
                <input
                  type="text"
                  value={selectedNode.subtitle || ''}
                  onChange={(e) => {
                    const subtitle = e.target.value;
                    setNodes((prev) => prev.map((n) => (n.id === selectedNode.id ? { ...n, subtitle } : n)));
                  }}
                />
              </label>

              <div className="inspector-info-box">
                <strong>Node Type:</strong> {selectedNode.type.toUpperCase()}<br />
                <strong>Coordinates:</strong> X: {selectedNode.x}, Y: {selectedNode.y}
              </div>

              {selectedNode.type === 'warehouse' && (
                <div className="inspector-section">
                  <h4>Warehouse Settings</h4>
                  <label className="inspector-checkbox">
                    <input type="checkbox" defaultChecked /> Require Manager PIN for Stock Adjustments
                  </label>
                </div>
              )}

              {selectedNode.type === 'workspace' && (
                <div className="inspector-section">
                  <h4>Workspace Access</h4>
                  <label className="inspector-checkbox">
                    <input type="checkbox" defaultChecked /> Allow Cashiers to Void Items
                  </label>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
