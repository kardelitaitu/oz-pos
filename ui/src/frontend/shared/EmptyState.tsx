// ui/src/frontend/shared/EmptyState.tsx
//
// ERR-03 consolidation: this is a compatibility re-export of the canonical
// `@/components/EmptyState` implementation. The project must have exactly one
// EmptyState source of truth. Do not reimplement the component here — see
// src/__tests__/errorPrimitivesImportPolicy.test.ts.

export { EmptyState } from '@/components/EmptyState';
export type { EmptyStateProps, EmptyStateRegion } from '@/components/EmptyState';
