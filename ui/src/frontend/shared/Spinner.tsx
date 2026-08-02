// ui/src/frontend/shared/Spinner.tsx
//
// ERR-03 consolidation: this is a compatibility re-export of the canonical
// `@/components/Spinner` implementation. The project must have exactly one
// Spinner source of truth. Do not reimplement the component here — see
// src/__tests__/errorPrimitivesImportPolicy.test.ts.

export { Spinner } from '@/components/Spinner';
export type { SpinnerProps, SpinnerSize } from '@/components/Spinner';
