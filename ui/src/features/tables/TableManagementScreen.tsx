import { useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '@/components/Button';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import {
  listTablesScoped,
  listSectionsScoped,
  updateTableStatusScoped,
  releaseTableScoped,
  type Table,
} from '@/api/tables';
import './TableManagementScreen.css';

/** Table management screen — interactive floor-plan view for managing restaurant table status (available, occupied, reserved, cleaning). */
export default function TableManagementScreen() {
  const { l10n } = useLocalization();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken || '';
  const [tables, setTables] = useState<Table[]>([]);
  const [sections, setSections] = useState<string[]>([]);
  const [selected, setSelected] = useState<Table | null>(null);
  const [section, setSection] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  // Request-generation guard (TBL-02): a stale response from an earlier
  // section/token/refresh can never overwrite a fresher result.
  const loadSeqRef = useRef(0);

  // TBL-09: sections are stable metadata loaded independently of the table
  // page, so selecting a section never makes the other filters disappear and
  // empty sections stay representable. Non-fatal — the floor plan still works
  // if section loading fails.
  useEffect(() => {
    if (!sessionToken) return;
    let cancelled = false;
    listSectionsScoped(sessionToken)
      .then((data) => {
        if (!cancelled) setSections(data);
      })
      .catch(() => {
        if (!cancelled) setSections([]);
      });
    return () => {
      cancelled = true;
    };
  }, [sessionToken]);

  // TBL-02: durable loading/error/retry with a seq guard. Known-good tables
  // are preserved during a refresh (the floor plan stays visible); failures
  // surface a localized error and a retry path.
  const loadTables = useCallback(async () => {
    const seq = ++loadSeqRef.current;
    setLoading(true);
    setError(null);
    try {
      const data = await listTablesScoped(sessionToken, section ?? undefined);
      if (seq !== loadSeqRef.current) return;
      setTables(data);
    } catch (err) {
      if (seq !== loadSeqRef.current) return;
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      if (seq === loadSeqRef.current) setLoading(false);
    }
  }, [sessionToken, section]);

  useEffect(() => {
    if (!sessionToken) return;
    void loadTables();
  }, [loadTables, sessionToken, refreshKey]);

  const retry = useCallback(() => setRefreshKey((k) => k + 1), []);

  const showEmpty = !loading && !error && tables.length === 0;

  return (
    <div className="tables" role="region" aria-label={l10n.getString('tables-management-label')}>
      <h1 className="tables-title"><Localized id="tables-title">Table Management</Localized></h1>
      <div className="tables-sections">
        <Button variant="ghost" size="sm" className={`tables-section-btn ${section === null ? 'active' : ''}`}
          onClick={() => setSection(null)}><Localized id="tables-all">All</Localized></Button>
        {sections.map(s => (
          <Button variant="ghost" size="sm" key={s} className={`tables-section-btn ${section === s ? 'active' : ''}`}
            onClick={() => setSection(s)}>{s}</Button>
        ))}
      </div>

      {error && (
        <div className="tables-error" role="alert">
          <span className="tables-error-text">
            <Localized id="tables-load-error">Could not load the floor plan.</Localized>
            {error && <span className="tables-error-detail">{error}</span>}
          </span>
          <Button variant="primary" size="sm" onClick={retry}>
            <Localized id="retry">Retry</Localized>
          </Button>
        </div>
      )}

      <div className="tables-floorplan" role="list" aria-label={l10n.getString('tables-floorplan-label')}>
        {loading && tables.length === 0 && (
          <div className="tables-loading" role="status">
            <span className="tables-loading-spinner" aria-hidden="true" />
            <Localized id="loading">Loading…</Localized>
          </div>
        )}

        {showEmpty && section === null && (
          <div className="tables-empty">
            <span className="tables-empty-icon" aria-hidden="true">▦</span>
            <p className="tables-empty-title"><Localized id="tables-empty">No tables configured yet.</Localized></p>
            <p className="tables-empty-desc"><Localized id="tables-empty-desc">Add tables from the settings screen to build your floor plan.</Localized></p>
          </div>
        )}

        {showEmpty && section !== null && (
          <div className="tables-empty">
            <span className="tables-empty-icon" aria-hidden="true">▦</span>
            <p className="tables-empty-title"><Localized id="tables-empty-filtered">No tables in this section.</Localized></p>
            <Button variant="ghost" size="sm" onClick={() => setSection(null)}>
              <Localized id="tables-all">All</Localized>
            </Button>
          </div>
        )}

        {!showEmpty && tables.map((t) => {
          const shape = t.shape || 'circle';
          // TBL-08 front-end clamp: old persisted geometry could predate the
          // backend bounds check, so never render a sub-2% interactive control.
          const w = Math.max(t.width, 2);
          const h = Math.max(t.height, 2);
          return (
            <Button variant="ghost" size="sm" key={t.id} className={`tables-table tables-table--${t.status} tables-table--${shape}`}
              onClick={() => setSelected(t)}
              onContextMenu={(e) => { e.preventDefault(); statusAction(t, sessionToken); }}
              style={{
                left: `${t.pos_x}%`, top: `${t.pos_y}%`,
                width: `${w}%`, height: `${h}%`,
              }}
              aria-label={l10n.getString('tables-table-label', { name: t.name, status: t.status })}
            >
              <span className="tables-table-name">{t.name}</span>
              <span className="tables-table-status">{t.status}</span>
            </Button>
          );
        })}
      </div>

      {selected && (
        <div className="tables-detail" role="dialog" aria-label={l10n.getString('tables-detail-label')}>
          <h2>{selected.name}</h2>
          <p><Localized id="tables-capacity-label" vars={{ capacity: selected.capacity }}><span>Capacity: {selected.capacity}</span></Localized></p>
          <p><Localized id="tables-status-label" vars={{ status: selected.status }}><span>Status: {selected.status}</span></Localized></p>
          <p><Localized id="tables-section-label" vars={{ section: selected.section || '—' }}><span>Section: {selected.section || '—'}</span></Localized></p>
          <div className="tables-detail-actions">
            <Button variant={selected.status === 'occupied' ? 'danger' : 'primary'} size="sm" onClick={() => { statusAction(selected, sessionToken); setSelected(null); }}>
              <Localized id={selected.status === 'occupied' ? 'tables-release' : 'tables-mark-available'}>{selected.status === 'occupied' ? 'Release' : 'Mark Available'}</Localized>
            </Button>
            <Button variant="ghost" size="sm" onClick={() => setSelected(null)}>
              <Localized id="close">Close</Localized>
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

/**
 * Fire the status transition for a table (available→occupied / occupied→release /
 * reserved|cleaning→available). Phase 3 makes this async, pending-guarded, and
 * error-aware (TBL-03) and moves the quick action behind an accessible menu
 * instead of the context-menu-only shortcut (TBL-05).
 */
function statusAction(table: Table, sessionToken: string) {
  if (table.status === 'available') {
    updateTableStatusScoped(sessionToken, table.id, 'occupied');
  } else if (table.status === 'occupied') {
    releaseTableScoped(sessionToken, table.id);
  } else if (table.status === 'reserved') {
    updateTableStatusScoped(sessionToken, table.id, 'available');
  } else if (table.status === 'cleaning') {
    updateTableStatusScoped(sessionToken, table.id, 'available');
  }
}
