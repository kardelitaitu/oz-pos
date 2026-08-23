// ── ui/src/features/warehouse/useWarehouseSession.ts ─────────────────────
// Session state hook for the warehouse console (send/receive/count).
// Self-contained copy of the retail cart-state pattern, simplified:
// no modifiers, courses, tax, or customer — just lines, quantities,
// mode, and pick-verify state.

import { useCallback, useMemo, useRef, useState } from 'react';

export type WarehouseMode = 'receive' | 'send' | 'count' | 'stock';

/** A line in the warehouse session — one SKU with a quantity. */
export interface WarehouseSessionLine {
  /** Stable line id (uuid-ish). */
  id: string;
  sku: string;
  productName: string;
  /** Optional bin/rack hint (products.rack_location). */
  bin?: string | null;
  qty: number;
  /** Send mode: how many have been scan-verified as picked. */
  pickedQty: number;
  /** Receive mode: the real stock_transfer_line.id this line maps to. */
  transferLineId?: string;
}

export interface WarehouseSessionState {
  mode: WarehouseMode;
  lines: WarehouseSessionLine[];
  /** Send mode: destination location id (picked via dialog). */
  destinationLocationId: string | null;
  /** Receive mode: source transfer id (picked via dialog). */
  transferId: string | null;
  /** Receive mode: PO id (picked via dialog). */
  poId: string | null;
  setMode: (mode: WarehouseMode) => void;
  setDestinationLocationId: (id: string | null) => void;
  setTransferId: (id: string | null) => void;
  setPoId: (id: string | null) => void;
  addLine: (sku: string, productName: string, bin?: string | null, qty?: number, transferLineId?: string) => void;
  setQty: (lineId: string, qty: number) => void;
  /** Send mode: mark a line's pick-verify quantity. */
  pickLine: (lineId: string, pickedQty: number) => void;
  removeLine: (lineId: string) => void;
  clear: () => void;
  /** Total units across lines. */
  itemCount: number;
  /** Send mode: all lines fully picked. */
  fullyPicked: boolean;
  /** True when there are no lines. */
  isEmpty: boolean;
}

let seq = 0;
function newLineId(): string {
  seq += 1;
  return `whl-${Date.now().toString(36)}-${seq}`;
}

export function useWarehouseSession(): WarehouseSessionState {
  const [mode, setMode] = useState<WarehouseMode>('receive');
  const [lines, setLines] = useState<WarehouseSessionLine[]>([]);
  const [destinationLocationId, setDestinationLocationId] = useState<string | null>(null);
  const [transferId, setTransferId] = useState<string | null>(null);
  const [poId, setPoId] = useState<string | null>(null);
  const linesRef = useRef(lines);
  linesRef.current = lines;

  const addLine = useCallback(
    (sku: string, productName: string, bin?: string | null, qty = 1, transferLineId?: string) => {
      setLines((prev) => {
        const existing = prev.find((l) => l.sku === sku);
        if (existing) {
          return prev.map((l) =>
            l.sku === sku ? { ...l, qty: l.qty + qty } : l,
          );
        }
        return [
          ...prev,
          {
            id: newLineId(),
            sku,
            productName,
            bin: bin ?? null,
            qty,
            pickedQty: 0,
            ...(transferLineId ? { transferLineId } : {}),
          },
        ];
      });
    },
    [],
  );

  const setQty = useCallback((lineId: string, qty: number) => {
    setLines((prev) =>
      prev.map((l) => (l.id === lineId ? { ...l, qty: Math.max(0, qty) } : l)),
    );
  }, []);

  const pickLine = useCallback((lineId: string, pickedQty: number) => {
    setLines((prev) =>
      prev.map((l) =>
        l.id === lineId ? { ...l, pickedQty: Math.max(0, pickedQty) } : l,
      ),
    );
  }, []);

  const removeLine = useCallback((lineId: string) => {
    setLines((prev) => prev.filter((l) => l.id !== lineId));
  }, []);

  const clear = useCallback(() => {
    setLines([]);
    setDestinationLocationId(null);
    setTransferId(null);
    setPoId(null);
  }, []);

  const itemCount = useMemo(
    () => lines.reduce((sum, l) => sum + l.qty, 0),
    [lines],
  );

  const fullyPicked = useMemo(
    () => lines.length > 0 && lines.every((l) => l.pickedQty >= l.qty),
    [lines],
  );

  const isEmpty = lines.length === 0;

  return {
    mode,
    lines,
    destinationLocationId,
    transferId,
    poId,
    setMode,
    setDestinationLocationId,
    setTransferId,
    setPoId,
    addLine,
    setQty,
    pickLine,
    removeLine,
    clear,
    itemCount,
    fullyPicked,
    isEmpty,
  };
}