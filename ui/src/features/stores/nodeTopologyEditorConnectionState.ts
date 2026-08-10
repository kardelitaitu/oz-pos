import { useCallback, useReducer } from 'react';
import type { PortName } from './NodeTopologyEditor';
import type { WireRelationshipOption } from './topologyCard';

/**
 * Typed state machine for the topology editor's in-flight wire connection
 * and its relationship picker (ADR #34).
 *
 * The two are one gesture: a connection is armed from a source port, and
 * when the drop admits MULTIPLE relationships the picker opens while the
 * connection stays visible (ghost + source highlight) until the user picks
 * one, cancels, or abandons it.
 *
 * Before this module, the picker and the connection were separate useStates
 * with hand-rolled cleanup that disagreed: Escape and the picker's Cancel
 * button went through `cancelRelationshipPicker` (cleared BOTH), but
 * dismissing the picker via canvas click, drag start, or touch cleared only
 * `setRelationshipPicker(null)` — leaving the armed connection alive, so a
 * later port click could complete a wire from the stale source. The reducer
 * makes the invariant structural: `cancel` always clears both, and `begin`
 * always closes any open picker.
 */
export type TopologyPickerState = {
  fromNodeId: string;
  fromPort: PortName;
  toNodeId: string;
  toPort: PortName;
  options: WireRelationshipOption[];
};

export type TopologyConnectionState = {
  /** Source node of the in-flight connection, or null. */
  fromNodeId: string | null;
  /** Source port of the in-flight connection, or null. */
  fromPort: PortName | null;
  /** Open relationship picker, or null. */
  picker: TopologyPickerState | null;
};

export type TopologyConnectionAction =
  | { type: 'begin'; fromNodeId: string; fromPort: PortName }
  | { type: 'open-picker'; picker: TopologyPickerState }
  | { type: 'cancel' }
  | { type: 'dismiss-picker' };

export const initialTopologyConnectionState: TopologyConnectionState = {
  fromNodeId: null,
  fromPort: null,
  picker: null,
};

export function topologyConnectionReducer(
  state: TopologyConnectionState,
  action: TopologyConnectionAction,
): TopologyConnectionState {
  switch (action.type) {
    case 'begin':
      // A fresh connection attempt always closes any open picker — the old
      // pending choice belongs to the abandoned gesture.
      return { fromNodeId: action.fromNodeId, fromPort: action.fromPort, picker: null };
    case 'open-picker':
      return { ...state, picker: action.picker };
    case 'cancel':
      // Clearing the picker ALWAYS clears the armed connection too — the
      // ghost and stale source must not survive a dismissed choice.
      return { fromNodeId: null, fromPort: null, picker: null };
    case 'dismiss-picker':
      // Dismissing an OPEN picker is a full cancel (same as Escape / the
      // Cancel button); a plain armed connection with no picker survives a
      // canvas click — the user may be panning to a distant target.
      return state.picker
        ? { fromNodeId: null, fromPort: null, picker: null }
        : state;
  }
}

/**
 * Connection/picker state boundary for the editor. Exposes the same field
 * names the component already consumes (`fromNodeId` / `fromPort` /
 * `picker`) plus atomic transitions: `beginConnection`, `openPicker`, and
 * `cancelConnection` (the invariant-enforcing dismissal).
 */
export function useTopologyEditorConnection() {
  const [state, dispatch] = useReducer(topologyConnectionReducer, initialTopologyConnectionState);

  const beginConnection = useCallback((fromNodeId: string, fromPort: PortName) => {
    dispatch({ type: 'begin', fromNodeId, fromPort });
  }, []);
  const openPicker = useCallback((picker: TopologyPickerState) => {
    dispatch({ type: 'open-picker', picker });
  }, []);
  const cancelConnection = useCallback(() => {
    dispatch({ type: 'cancel' });
  }, []);
  const dismissPicker = useCallback(() => {
    dispatch({ type: 'dismiss-picker' });
  }, []);

  return {
    ...state,
    beginConnection,
    openPicker,
    cancelConnection,
    dismissPicker,
  };
}
