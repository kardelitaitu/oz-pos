import { loggedInvoke } from '@/utils/logged-invoke';

/** A stock transfer between locations or terminals. */
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

/** A line item within a stock transfer. */
export interface StockTransferLine {
  id: string;
  transfer_id: string;
  sku: string;
  product_name: string;
  qty: number;
  received_qty: number;
}

/** A stock transfer with its line items. */
export interface TransferWithLines {
  transfer: StockTransfer;
  lines: StockTransferLine[];
}

/** Input for recording the received quantity of a transfer line. */
export interface ReceivedLineInput {
  line_id: string;
  received_qty: number;
}

/** Create a new stock transfer between locations or terminals. */
export const createStockTransfer = (
  sourceLocation: string | null,
  destinationLocation: string | null,
  sourceTerminalId: string | null,
  destinationTerminalId: string | null,
  notes: string,
  createdBy: string,
  lines: StockTransferLine[],
): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('create_stock_transfer', {
    sourceLocation,
    destinationLocation,
    sourceTerminalId,
    destinationTerminalId,
    notes,
    createdBy,
    lines,
  });

/** Get a single stock transfer by its identifier. */
export const getStockTransfer = (id: string): Promise<TransferWithLines | null> =>
  loggedInvoke<TransferWithLines | null>('get_stock_transfer', { id });

/** List all stock transfers. */
export const listStockTransfers = (): Promise<StockTransfer[]> =>
  loggedInvoke<StockTransfer[]>('list_stock_transfers');

/** Get all line items for a stock transfer. */
export const getStockTransferLines = (transferId: string): Promise<StockTransferLine[]> =>
  loggedInvoke<StockTransferLine[]>('get_stock_transfer_lines', { transferId });

/** Add a line item to a stock transfer. */
export const addStockTransferLine = (
  transferId: string,
  sku: string,
  productName: string,
  qty: number,
): Promise<StockTransferLine> =>
  loggedInvoke<StockTransferLine>('add_stock_transfer_line', {
    transferId,
    sku,
    productName,
    qty,
  });

/** Remove a line item from a stock transfer. */
export const removeStockTransferLine = (lineId: string): Promise<void> =>
  loggedInvoke<void>('remove_stock_transfer_line', { lineId });

/** Mark a stock transfer as sent (dispatched from source). */
export const sendStockTransfer = (id: string): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('send_stock_transfer', { id });

/** Mark a stock transfer as received, updating quantities for each line. */
export const receiveStockTransfer = (
  id: string,
  receivedBy: string,
  receivedLines: ReceivedLineInput[],
): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('receive_stock_transfer', { id, receivedBy, receivedLines });

/** Cancel a stock transfer. */
export const cancelStockTransfer = (id: string): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('cancel_stock_transfer', { id });
