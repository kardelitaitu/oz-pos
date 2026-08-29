## Summary

Polish the marketing website with brand-correct platform icons, a tech-stack showcase section, refined download page layout, and updated i18n strings. Also removes the archived multi-agent-orchestrator skill.

## Key Changes

### 1. Website — Platform Icons (PlatformIcon.astro)
- Android platform icon now reveals its real brand colours on hover (`group-hover`): a full-colour `<img>` sits underneath the CSS mask; on hover the mask fades out and the coloured logo appears.
- Windows icon keeps the existing `currentColor` mask approach.
- Refactored the component to split Android vs. non-Android rendering paths.

### 2. Website — Homepage Tech Stack Section (index.astro)
- Added a new "Built on a modern tech stack" section after the features grid displaying brand logos: Rust, Tauri, Paddle, Midtrans, Trivy, PostgreSQL.
- Logos are lazy-loaded, responsive (`flex-wrap`), and have hover opacity transitions.

### 3. Website — Download Page Layout (download.astro)
- Platform cards now use `flex-col` with `mt-auto` on the CTA button so cards align evenly regardless of label length.
- CTA buttons use `flex h-11` for consistent 44px touch-target sizing.
- Renamed `os` field to `label` for clarity; updated labels (e.g. "Linux (glibc 2.31+)", "Android (Tablets & phones)").

### 4. i18n — English & Indonesian Translations
- Added `techStack.title` and `techStack.subtitle` keys in both `en.json` and `id.json`.
- Updated system requirements strings: macOS minimum lowered to 10.13, Android 7.0+, iOS 14.0+ (was generic "tablets & phones").

### 5. Cleanup — Remove Archived Skill
- Deleted `.agents/skills/multi-agent-orchestrator/SKILL.md` (archived/orphaned skill no longer referenced).

## Commit History Highlights
- `3f33bea0` — `feat(website): brand logo hover effects, tech-stack logos, and i18n updates`: All changes in this PR.

## Verification & Testing
- [x] Website link checker: `cd website && node scripts/check-links.mjs` — 0 broken links
- [x] i18n lint: no issues detected (pre-commit gate)
- [x] UI typecheck: `npm run typecheck` — passed

## Files Changed
- `website/src/components/PlatformIcon.astro` — Android hover brand-colour effect
- `website/src/pages/[locale]/index.astro` — Tech-stack logo section
- `website/src/pages/[locale]/download.astro` — Platform card layout + touch targets
- `website/src/i18n/en.json` — New keys + updated requirements
- `website/src/i18n/id.json` — New keys + updated requirements (Indonesian)
- `.agents/skills/multi-agent-orchestrator/SKILL.md` — Deleted
