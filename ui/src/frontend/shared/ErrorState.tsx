// ui/src/frontend/shared/ErrorState.tsx
//
// ERR-03 consolidation: this is a compatibility re-export of the canonical
// `@/components/ErrorState` implementation. The project must have exactly one
// ErrorState source of truth so accessibility/retry fixes reach every
// consumer. Do not reimplement the component here — see
// src/__tests__/errorPrimitivesImportPolicy.test.ts.

export { ErrorState } from '@/components/ErrorState';
export type { ErrorStateProps } from '@/components/ErrorState';
