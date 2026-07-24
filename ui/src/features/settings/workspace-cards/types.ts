/** Variant controlling card width, padding, and chrome. */
export type WorkspaceCardVariant = 'full-page' | 'modal' | 'inspector-drawer';

/** Shared props interface consumed by all workspace settings cards. */
export interface WorkspaceCardProps {
  /** Session token for authenticated API calls. */
  sessionToken?: string;
  /** Inventory location ID scoping deduction rules. */
  locationId?: string;
  /**
   * Terminal ID for register-local hardware bindings.
   * Required when `variant='modal'` so the card knows which register's
   * hardware to display.
   */
  terminalId?: string;
  /** Rendering context controlling width, padding, and header bar. */
  variant?: WorkspaceCardVariant;
  /** Fired after successful save — parent can dismiss modal or refresh. */
  onSaved?: () => void;
}
