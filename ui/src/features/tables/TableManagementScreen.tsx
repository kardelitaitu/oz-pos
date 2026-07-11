import { useEffect, useState } from 'react';
import { Localized, useLocalization } from '@fluent/react';
import { useAuth } from '@/contexts/AuthContext';
import { listTables, updateTableStatus, releaseTable, type Table } from '@/api/tables';
import './TableManagementScreen.css';

/** Table management screen — interactive floor-plan view for managing restaurant table status (available, occupied, reserved, cleaning). */
export default function TableManagementScreen() {
  const { l10n } = useLocalization();
  const { session } = useAuth();
  const [tables, setTables] = useState<Table[]>([]);
  const [selected, setSelected] = useState<Table | null>(null);
  const [section, setSection] = useState<string | null>(null);

  useEffect(() => {
    listTables(section ?? undefined).then(setTables);
  }, [section]);

  const userId = session?.user_id ?? '';

  const statusAction = (table: Table) => {
    if (table.status === 'available') {
      updateTableStatus(userId, table.id, 'occupied');
    } else if (table.status === 'occupied') {
      releaseTable(userId, table.id);
    } else if (table.status === 'reserved') {
      updateTableStatus(userId, table.id, 'available');
    } else if (table.status === 'cleaning') {
      updateTableStatus(userId, table.id, 'available');
    }
  };

  return (
    <div className="tables" role="region" aria-label={l10n.getString('tables-management-label')}>
      <h1 className="tables-title"><Localized id="tables-title">Table Management</Localized></h1>
      <div className="tables-sections">
        <button className={`tables-section-btn ${section === null ? 'active' : ''}`}
          onClick={() => setSection(null)}><Localized id="tables-all">All</Localized></button>
        {[...new Set(tables.map(t => t.section).filter(Boolean))].map(s => (
          <button key={s} className={`tables-section-btn ${section === s ? 'active' : ''}`}
            onClick={() => setSection(s)}>{s}</button>
        ))}
      </div>
      <div className="tables-floorplan" role="list" aria-label={l10n.getString('tables-floorplan-label')}>
        {tables.map((t) => {
          const shape = t.shape || 'circle';
          return (
            <button key={t.id} className={`tables-table tables-table--${t.status} tables-table--${shape}`}
              onClick={() => setSelected(t)}
              onContextMenu={(e) => { e.preventDefault(); statusAction(t); }}
              style={{
                left: `${t.pos_x}%`, top: `${t.pos_y}%`,
                width: `${t.width}%`, height: `${t.height}%`,
              }}
              aria-label={l10n.getString('tables-table-label', { name: t.name, status: t.status })}
            >
              <span className="tables-table-name">{t.name}</span>
              <span className="tables-table-status">{t.status}</span>
            </button>
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
            <button onClick={() => { statusAction(selected); setSelected(null); }}>
              <Localized id={selected.status === 'occupied' ? 'tables-release' : 'tables-mark-available'}>{selected.status === 'occupied' ? 'Release' : 'Mark Available'}</Localized>
            </button>
            <button onClick={() => setSelected(null)}><Localized id="close">Close</Localized></button>
          </div>
        </div>
      )}
    </div>
  );
}
