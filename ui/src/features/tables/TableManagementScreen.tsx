import { useEffect, useState } from 'react';
import { Localized } from '@fluent/react';
import { listTables, updateTableStatus, releaseTable, type Table } from '@/api/tables';
import './TableManagementScreen.css';

const STATUS_COLORS: Record<string, string> = {
  available: '#22c55e',
  occupied: '#ef4444',
  reserved: '#eab308',
  cleaning: '#9ca3af',
};

export default function TableManagementScreen() {
  const [tables, setTables] = useState<Table[]>([]);
  const [selected, setSelected] = useState<Table | null>(null);
  const [section, setSection] = useState<string | null>(null);

  useEffect(() => {
    listTables(section ?? undefined).then(setTables);
  }, [section]);

  const statusAction = (table: Table) => {
    if (table.status === 'available') {
      updateTableStatus(table.id, 'occupied');
    } else if (table.status === 'occupied') {
      releaseTable(table.id);
    } else if (table.status === 'reserved') {
      updateTableStatus(table.id, 'available');
    } else if (table.status === 'cleaning') {
      updateTableStatus(table.id, 'available');
    }
  };

  return (
    <div className="tables" role="region" aria-label="Table management">
      <h1 className="tables-title"><Localized id="tables-title">Table Management</Localized></h1>
      <div className="tables-sections">
        <button className={`tables-section-btn ${section === null ? 'active' : ''}`}
          onClick={() => setSection(null)}><Localized id="tables-all">All</Localized></button>
        {[...new Set(tables.map(t => t.section).filter(Boolean))].map(s => (
          <button key={s} className={`tables-section-btn ${section === s ? 'active' : ''}`}
            onClick={() => setSection(s)}>{s}</button>
        ))}
      </div>
      <div className="tables-floorplan" role="list" aria-label="Floor plan">
        {tables.map((t) => (
          <button key={t.id} className="tables-table"
            onClick={() => setSelected(t)}
            onContextMenu={(e) => { e.preventDefault(); statusAction(t); }}
            style={{
              left: `${t.pos_x}%`, top: `${t.pos_y}%`,
              width: `${t.width}%`, height: `${t.height}%`,
              backgroundColor: STATUS_COLORS[t.status] ?? '#6b7280',
            }}
            aria-label={`${t.name}, ${t.status}`}
          >
            <span className="tables-table-name">{t.name}</span>
            <span className="tables-table-status">{t.status}</span>
          </button>
        ))}
      </div>
      {selected && (
        <div className="tables-detail" role="dialog" aria-label="Table detail">
          <h2>{selected.name}</h2>
          <p>Capacity: {selected.capacity}</p>
          <p>Status: {selected.status}</p>
          <p>Section: {selected.section || '—'}</p>
          <div className="tables-detail-actions">
            <button onClick={() => { statusAction(selected); setSelected(null); }}>
              {selected.status === 'occupied' ? 'Release' : 'Mark Available'}
            </button>
            <button onClick={() => setSelected(null)}><Localized id="close">Close</Localized></button>
          </div>
        </div>
      )}
    </div>
  );
}
