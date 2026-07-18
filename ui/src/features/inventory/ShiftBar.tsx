import { useState, useEffect, useRef } from 'react';
import { Localized } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import {
  startInventoryShift,
  endInventoryShift,
  getActiveInventoryShift,
  listInventoryLocations,
  listInventoryTransactions,
  type InventoryShift,
  type InventoryLocation,
  type InventoryTransaction,
} from '@/api/inventory';
import './ShiftBar.css';

interface ShiftBarProps {
  onShiftChange?: (shift: InventoryShift | null) => void;
}

export default function ShiftBar({ onShiftChange }: ShiftBarProps) {
  const { session } = useAuth();
  const { sessionToken } = useWorkspace();

  const [activeShift, setActiveShift] = useState<InventoryShift | null>(null);
  const [locations, setLocations] = useState<InventoryLocation[]>([]);
  const [selectedLocationId, setSelectedLocationId] = useState('');
  const [notes, setNotes] = useState('');
  
  // Timer state
  const [elapsedText, setElapsedText] = useState('00:00:00');
  
  // Summary modal state
  const [showSummary, setShowSummary] = useState(false);
  const [shiftSummaryTxs, setShiftSummaryTxs] = useState<InventoryTransaction[]>([]);

  const timerRef = useRef<NodeJS.Timeout | null>(null);

  // Load locations and active shift
  useEffect(() => {
    if (!sessionToken || !session?.user_id) return;

    listInventoryLocations(sessionToken)
      .then(locs => {
        const activeLocs = locs.filter(l => l.is_active);
        setLocations(activeLocs);
        if (activeLocs.length > 0) {
          setSelectedLocationId(activeLocs[0]!.id);
        }
      })
      .catch(console.error);

    getActiveInventoryShift(sessionToken, session.user_id)
      .then(shift => {
        setActiveShift(shift);
        if (onShiftChange) onShiftChange(shift);
      })
      .catch(console.error);
  }, [sessionToken, session?.user_id]);

  // Handle timer tick
  useEffect(() => {
    if (activeShift) {
      const updateTimer = () => {
        const start = new Date(activeShift.started_at).getTime();
        const now = Date.now();
        const diff = Math.max(0, now - start);
        
        const hrs = Math.floor(diff / 3600000);
        const mins = Math.floor((diff % 3600000) / 60000);
        const secs = Math.floor((diff % 6000) / 1000);
        
        const pad = (n: number) => n.toString().padStart(2, '0');
        setElapsedText(`${pad(hrs)}:${pad(mins)}:${pad(secs)}`);
      };

      updateTimer();
      timerRef.current = setInterval(updateTimer, 1000);
    } else {
      setElapsedText('00:00:00');
    }

    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [activeShift]);

  const handleStartShift = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!sessionToken || !session?.user_id || !selectedLocationId) return;

    try {
      const shift = await startInventoryShift(sessionToken, session.user_id, selectedLocationId, notes);
      setActiveShift(shift);
      setNotes('');
      if (onShiftChange) onShiftChange(shift);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to start shift');
    }
  };

  const handleEndShift = async () => {
    if (!sessionToken || !activeShift) return;

    try {
      // Fetch transactions before ending to summarize activity
      const allTxs = await listInventoryTransactions(sessionToken);
      const startTime = new Date(activeShift.started_at).getTime();
      
      const shiftLocId = activeShift.location_id;
      const filtered: InventoryTransaction[] = [];
      for (const tx of allTxs) {
        const txTime = new Date(tx.created_at).getTime();
        const uid = session?.user_id;
        if (uid != null && tx.staff_id === uid &&
            tx.location_id === shiftLocId &&
            txTime >= startTime) {
          filtered.push(tx);
        }
      }

      await endInventoryShift(sessionToken, activeShift.id);
      setShiftSummaryTxs(filtered);
      setShowSummary(true);
      setActiveShift(null);
      if (onShiftChange) onShiftChange(null);
    } catch (err) {
      alert(err instanceof Error ? err.message : 'Failed to end shift');
    }
  };

  const activeLocationName = locations.find(l => l.id === activeShift?.location_id)?.name ?? activeShift?.location_id ?? '';

  return (
    <>
      <div className="inventory-shift-bar" role="region" aria-label="Shift Info">
        {activeShift ? (
          <div className="shift-status-active">
            <div className="status-indicator" />
            <span className="shift-info-text">
              <Localized
                id="inv-shift-active-info"
                vars={{
                  user: session?.display_name ?? '',
                  location: activeLocationName ?? '',
                  time: elapsedText,
                }}
              >
                <span>Active Shift</span>
              </Localized>
            </span>
            <button className="shift-btn shift-btn-danger" onClick={handleEndShift}>
              <Localized id="inv-shift-end-btn">
                <span>End Shift</span>
              </Localized>
            </button>
          </div>
        ) : (
          <form className="shift-start-form" onSubmit={handleStartShift}>
            <span className="shift-form-title">
              <Localized id="inv-shift-start-title">
                <span>Start Inventory Shift</span>
              </Localized>
            </span>
            
            <select
              className="shift-select"
              value={selectedLocationId}
              onChange={e => setSelectedLocationId(e.target.value)}
              aria-label="Location"
            >
              {locations.map(loc => (
                <option key={loc.id} value={loc.id}>
                  {loc.name}
                </option>
              ))}
            </select>

            <Localized id="inv-shift-notes-placeholder" attrs={{ placeholder: true }}>
              <input
                className="shift-input"
                type="text"
                value={notes}
                onChange={e => setNotes(e.target.value)}
                placeholder="Shift Notes"
                aria-label="Notes"
              />
            </Localized>

            <button type="submit" className="shift-btn shift-btn-primary">
              <Localized id="inv-shift-start-btn">
                <span>Start Shift</span>
              </Localized>
            </button>
          </form>
        )}
      </div>

      {showSummary && (
        <div className="shift-summary-overlay">
          <div className="shift-summary-modal" role="dialog" aria-modal="true">
            <Localized id="inv-shift-summary-title">
              <h3>Shift Summary</h3>
            </Localized>
            <Localized id="inv-shift-summary-performed">
              <p>Transactions performed during this shift:</p>
            </Localized>
            
            <ul className="summary-list">
              {shiftSummaryTxs.length > 0 ? (
                shiftSummaryTxs.map(tx => (
                  <li key={tx.id} className="summary-item">
                    <span style={{ textTransform: 'capitalize' }}>
                      {tx.type.replace('-', ' ')}
                    </span>
                    <span>{new Date(tx.created_at).toLocaleTimeString()}</span>
                  </li>
                ))
              ) : (
                <Localized id="inv-shift-no-transactions">
                  <li className="summary-item" style={{ borderLeftColor: '#ef4444' }}>
                    No transactions recorded.
                  </li>
                </Localized>
              )}
            </ul>

            <button className="shift-btn shift-btn-primary summary-close-btn" onClick={() => setShowSummary(false)}>
              <Localized id="inv-cancel">
                <span>Close</span>
              </Localized>
            </button>
          </div>
        </div>
      )}
    </>
  );
}
