import { useContext, type ReactNode } from 'react';
import { useLocalization } from '@fluent/react';
import { LocaleContext } from '@/i18n/LocaleContext';
import { openUpgradePricing, type UpgradeTarget } from '@/utils/upgrade';
import { Button } from '@/components/Button';
import './TierLockedFeature.css';

interface TierLockedFeatureProps {
  /** FTL key for the locked headline. */
  titleKey: string;
  /** FTL key for the explanation message. */
  messageKey: string;
  /** FTL key for the upgrade CTA button. */
  ctaKey: string;
  /** Pricing-page anchor the CTA deep-links to. */
  target: UpgradeTarget;
  /** Optional blurred preview shown behind/below the message (C2.2). */
  children?: ReactNode;
}

/**
 * C2.2: shared locked screen for tier-gated features (analytics, loyalty).
 * The feature stays visible as a blurred, non-interactive preview with an
 * upgrade CTA deep-linking to the matching tier on the pricing page.
 */
export default function TierLockedFeature({
  titleKey,
  messageKey,
  ctaKey,
  target,
  children,
}: TierLockedFeatureProps) {
  const { l10n } = useLocalization();
  // Tests render without LocaleContext, so default to English there.
  const locale = useContext(LocaleContext)?.locale ?? 'en';

  return (
    <section className="tier-locked" aria-label={l10n.getString(titleKey)}>
      <div className="tier-locked-content">
        <h2 className="tier-locked-title">{l10n.getString(titleKey)}</h2>
        <p className="tier-locked-message">{l10n.getString(messageKey)}</p>
        <Button variant="primary" onClick={() => openUpgradePricing(locale, target)}>
          {l10n.getString(ctaKey)}
        </Button>
      </div>
      {children && (
        <div className="tier-locked-preview" aria-hidden="true">
          {children}
        </div>
      )}
    </section>
  );
}
