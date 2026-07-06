import { invoke } from '@tauri-apps/api/core';

export interface StockTransfer {
  id: string;
  transfer_number: string;
  status: string;
  source_location: string | null;
  destination_location: string | null;
  source_terminal_id: string | null;
  destination_terminal_id: string | null;
  notes: string;
  created_by: string;
  received_by: string | null;
  created_at: string;
  sent_at: string | null;
  received_at: string | null;
  updated_at: string;
}

export interface StockTransferLine {
  id: string;
  transfer_id: string;
  sku: string;
  product_name: string;
  qty: number;
  received_qty: number;
}

export interface TransferWithLines {
  transfer: StockTransfer;
  lines: StockTransferLine[];
}

export interface ReceivedLineInput {
  line_id: string;
  received_qty: number;
}

export const createStockTransfer = (
  sourceLocation: string | null,
  destinationLocation: string | null,
  sourceTerminalId: string | null,
  destinationTerminalId: string | null,
  notes: string,
  createdBy: string,
  lines: StockTransferLine[],
): Promise<StockTransfer> =>
  invoke<StockTransfer>('create_stock_transfer', {
    sourceLocation,
    destinationLocation,
    sourceTerminalId,
    destinationTerminalId,
    notes,
    createdBy,
    lines,
  });

export const getStockTransfer = (id: string): Promise<TransferWithLines | null> =>
  invoke<TransferWithLines | null>('get_stock_transfer', { id });

export const listStockTransfers = (): Promise<StockTransfer[]> =>
  invoke<StockTransfer[]>('list_stock_transfers');

export const getStockTransferLines = (transferId: string): Promise<StockTransferLine[]> =>
  invoke<StockTransferLine[]>('get_stock_transfer_lines', { transferId });

export const addStockTransferLine = (
  transferId: string,
  sku: string,
  productName: string,
  qty: number,
): Promise<StockTransferLine> =>
  invoke<StockTransferLine>('add_stock_transfer_line', {
    transferId,
    sku,
    productName,
    qty,
  });

export const removeStockTransferLine = (lineId: string): Promise<void> =>
  invoke<void>('remove_stock_transfer_line', { lineId });

export const sendStockTransfer = (id: string): Promise<StockTransfer> =>
  invoke<StockTransfer>('send_stock_transfer', { id });

export const receiveStockTransfer = (
  id: string,
  receivedBy: string,
  receivedLines: ReceivedLineInput[],
): Promise<StockTransfer> =>
  invoke<StockTransfer>('receive_stock_transfer', { id, receivedBy, receivedLines });

export const cancelStockTransfer = (id: string): Promise<StockTransfer> =>
  invoke<StockTransfer>('cancel_stock_transfer', { id });
