# i18n followup: 4 untranslated Indonesian bundles

When `feat(i18n): ...` lands, the survey of `ui/src/locales/*.id.ftl`
turned up **4 byte-identical-to-English** bundles. The user specifically
named `gift-cards.id.ftl` + `purchasing.id.ftl` as excluded; the survey
also caught `stock-counting.id.ftl` + `stock-transfers.id.ftl`. All 4 were
excluded from the commit per the principle "drop byte-identical" — shipping
a stub `.id.ftl` that is byte-identical to its `.ftl` sibling would present
**English text falsely labelled as Indonesian** rather than honestly falling
back. This file tracks the gap so translators + reviewers can prioritize.

## Bundles awaiting translation

| English source           | Indonesian stub (excluded)        | Affected screen(s)                                       | Est. effort |
| ------------------------ | --------------------------------- | -------------------------------------------------------- | ----------- |
| `gift-cards.ftl`         | `gift-cards.id.ftl`               | `GiftCardsScreen`, `IssueGiftCardModal`                  | ~30 min     |
| `purchasing.ftl`         | `purchasing.id.ftl`               | `PurchaseOrdersScreen`, `PurchaseOrderForm`, `SuppliersScreen` | ~45 min |
| `stock-counting.ftl`     | `stock-counting.id.ftl`           | `StockCountsScreen`, `StockCountDetail`, `StockCountHistory`    | ~30 min |
| `stock-transfers.ftl`    | `stock-transfers.id.ftl`          | `StockTransfersScreen`                                  | ~15 min     |

(20 other `.id.ftl` bundles are correctly translated and ship in the
`feat(i18n):` commit.)

## Why these are excluded specifically

The fallback path for `@fluent/react` is: if a key is missing from the
locale bundle (or the bundle file itself is absent), the next-most-relevant
fallback is used (typically the `en` bundle). This is a clean, well-tested
UX. A byte-identical `.id.ftl` loses this — it claims a translation that
doesn't exist, and the user gets English text presented under the
Indonesian flag, which is worse than the fallback happening transparently.

## Acceptance criteria for each translation PR

A translation PR lands once ALL of the following are true:

0. **Scaffolded properly.** Run `python3 scripts/translate-stub.py --bundle=<name> --write`
   to generate scaffolding. The script injects a `# HINT:` comment at the
   top with the per-domain translator guidance, prepends `[TODO]` to every
   value, and verifies the scaffold is NOT byte-identical to the source
   before writing to disk. Required because shipping the byte-identical
   `.id.ftl` would render English content under the Indonesian locale
   tag — worse than the runtime `[i18n]` fallback warning. The scaffold's
   dry-run (`--dry-run`, the default) is the recommended first step so
   the translator can preview what will be written.

1. **Same key set as the `.ftl` sibling.** No missing or extra keys.
   Verified by exit 0 from `scripts/verify-bundle-parity.py --staged-only`.
2. **Variable references preserved verbatim.** `{ $count }`, `{ $name }`,
   `{ $date }` etc. must match the source bundle character-for-character.
   The scaffold script protects these automatically; `scripts/translate-stub.py`
   has its own `validate_scaffold()` pass that catches placeholder drift
   before write.
3. **Multi-line values use the Fluent continuation form correctly.**
   A source value like

   ```
   long-text = {""first line
   second line with { $variable }""}
   ```

   requires leading-space indentation on continuation lines.

4. **No `[i18n]` lint warnings.** `bash scripts/lint-i18n.sh` exits 0
   without print the byte-identical sentinel — that no longer triggers
   because the bytes now differ. Also run `bash scripts/lint-i18n.sh`
   pre-push — it warns (yellow `[i18n]` prefix) if you ship a bundle
   whose bytes still match the English source.

5. **Indonesian-appropriate content.** Formal Indonesian uses Latin
   script; tooling does not require any specific UTF-8 encoding.

When a translation PR lands, update this file (remove the resolved table
row + ticket) and run `git commit --amend` on the feat(i18n) commit, OR
add a followup `fix(i18n): translate <bundle>` commit that closes out
the audit entry.

## Translator pointers

Translation PRs target `ui/src/locales/<bundle>.id.ftl`. The
acceptance criteria above are the de-facto submission contract; ping
the brand review team in the PR description so they can sign off on
copy-sensitive bundles (especially `gift-cards.id.ftl`).

The recommended workflow:

1. `python3 scripts/translate-stub.py --bundle=<name> --dry-run` to
   preview what the scaffold will look like.
2. `python3 scripts/translate-stub.py --bundle=<name> --write` once the
   preview looks right.
3. Open the resulting `ui/src/locales/<bundle>.id.ftl` and replace each
   `[TODO]` sentinel with the actual Indonesian translation.
4. Run `python3 scripts/verify-bundle-parity.py --staged-only` and
   `bash scripts/lint-i18n.sh` to confirm gates pass.

If a translator sees `@fluent/react` warnings at runtime after landing,
that's typically the bundle-parity gate rejecting an unmatched key; re-run
`python3 scripts/verify-bundle-parity.py --staged-only` and fix the
reported missing keys before re-merging. Also run `bash scripts/lint-i18n.sh`
pre-push — it warns (yellow `[i18n]` prefix) if you ship a bundle whose
bytes still match the English source.

## Total gap

4 bundles, ~2 hours of translation work to close. None are user-blocking;
the fallback path serves users with `locale="id"` correctly in the
interim.
