// ui/src/components/Skeleton.tsx
//
// LOAD-01: compatibility re-export. This path historically held a second,
// byte-identical copy of the Skeleton primitive, which allowed the two copies
// to drift (tokens, animation, a11y, API). The canonical implementation lives
// in `frontend/shared/Skeleton.tsx`; this path is kept so the 40+ existing
// `@/components/Skeleton` importers keep working against one source of truth.
export { Skeleton } from '../frontend/shared/Skeleton';
export type { SkeletonProps, SkeletonVariant } from '../frontend/shared/Skeleton';
