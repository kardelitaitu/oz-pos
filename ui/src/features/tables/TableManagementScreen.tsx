import { useEffect, useState } from 'react';
import { Button } from '@/components/Button';
import { Localized, useLocalization } from '@fluent/react';
import { useWorkspace } from '@/contexts/WorkspaceContext';
import { listTablesScoped, updateTableStatusScoped, releaseTableScoped, type Table } from '@/api/tables';
import './TableManagementScreen.css';

/** Table management screen — interactive floor-plan view for managing restaurant table status (available, occupied, reserved, cleaning). */
export default function TableManagementScreen() {
  const { l10n } = useLocalization();
  const { sessionToken: rawToken } = useWorkspace();
  const sessionToken = rawToken!;
  const [tables, setTables] = useState<Table[]>([]);
  const [selected, setSelected] = useState<Table | null>(null);
  const [section, setSection] = useState<string | null>(null);

  useEffect(() => {
    listTablesScoped(sessionToken, section ?? undefined).then(setTables);
  }, [sessionToken, section]);

  const statusAction = (table: Table) => {
    if (table.status === 'available') {
      updateTableStatusScoped(sessionToken, table.id, 'occupied');
    } else if (table.status === 'occupied') {
      releaseTableScoped(sessionToken, table.id);
    } else if (table.status === 'reserved') {
      updateTableStatusScoped(sessionToken, table.id, 'available');
    } else if (table.status === 'cleaning') {
      updateTableStatusScoped(sessionToken, table.id, 'available');
    }
  };

  return (
    <div className="tables" role="region" aria-label={l10n.getString('tables-management-label')}>
      <h1 className="tables-title"><Localized id="tables-title">Table Management</Localized></h1>
      <div className="tables-sections">
        <Button variant="ghost" size="sm" className={`tables-section-btn ${section === null ? 'active' : ''}`}
          onClick={() => setSection(null)}><Localized id="tables-all">All</Localized></Button>
        {[...new Set(tables.map(t => t.section).filter(Boolean))].map(s => (
          <Button variant="ghost" size="sm" key={s} className={`tables-section-btn ${section === s ? 'active' : ''}`}
            onClick={() => setSection(s)}>{s}</Button>
        ))}
      </div>
      <div className="tables-floorplan" role="list" aria-label={l10n.getString('tables-floorplan-label')}>
        {tables.map((t) => {
          const shape = t.shape || 'circle';
          return (
            <Button variant="ghost" size="sm" key={t.id} className={`tables-table tables-table--${t.status} tables-table--${shape}`}
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
            <Button variant={selected.status === 'occupied' ? 'danger' : 'primary'} size="sm" onClick={() => { statusAction(selected); setSelected(null); }}>
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
