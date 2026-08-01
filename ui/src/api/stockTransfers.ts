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

/** Create a stock transfer in the session-scoped store. */
export const createStockTransfer = (
  sessionToken: string,
  sourceLocation: string | null,
  destinationLocation: string | null,
  sourceTerminalId: string | null,
  destinationTerminalId: string | null,
  notes: string,
  lines: StockTransferLine[],
): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('create_stock_transfer_scoped', {
    sessionToken,
    sourceLocation,
    destinationLocation,
    sourceTerminalId,
    destinationTerminalId,
    notes,
    lines,
  });

/** Get a stock transfer from the session-scoped store. */
export const getStockTransfer = (
  sessionToken: string,
  id: string,
): Promise<TransferWithLines | null> =>
  loggedInvoke<TransferWithLines | null>('get_stock_transfer_scoped', { sessionToken, id });

/** List stock transfers from the session-scoped store. */
export const listStockTransfers = (sessionToken: string): Promise<StockTransfer[]> =>
  loggedInvoke<StockTransfer[]>('list_stock_transfers_scoped', { sessionToken });

/** Get all line items for a transfer in the session-scoped store. */
export const getStockTransferLines = (
  sessionToken: string,
  transferId: string,
): Promise<StockTransferLine[]> =>
  loggedInvoke<StockTransferLine[]>('get_stock_transfer_lines_scoped', {
    sessionToken,
    transferId,
  });

/** Add a line item to a draft transfer in the session-scoped store. */
export const addStockTransferLine = (
  sessionToken: string,
  transferId: string,
  sku: string,
  productName: string,
  qty: number,
): Promise<StockTransferLine> =>
  loggedInvoke<StockTransferLine>('add_stock_transfer_line_scoped', {
    sessionToken,
    transferId,
    sku,
    productName,
    qty,
  });

/** Remove a line item from a draft transfer in the session-scoped store. */
export const removeStockTransferLine = (
  sessionToken: string,
  lineId: string,
): Promise<void> =>
  loggedInvoke<void>('remove_stock_transfer_line_scoped', { sessionToken, lineId });

/** Mark a transfer as sent in the session-scoped store. */
export const sendStockTransfer = (
  sessionToken: string,
  id: string,
): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('send_stock_transfer_scoped', { sessionToken, id });

/** Mark a transfer as received; the backend derives received_by from the session. */
export const receiveStockTransfer = (
  sessionToken: string,
  id: string,
  receivedLines: ReceivedLineInput[],
): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('receive_stock_transfer_scoped', {
    sessionToken,
    id,
    receivedLines,
  });

/** Cancel a transfer in the session-scoped store. */
export const cancelStockTransfer = (
  sessionToken: string,
  id: string,
): Promise<StockTransfer> =>
  loggedInvoke<StockTransfer>('cancel_stock_transfer_scoped', { sessionToken, id });
