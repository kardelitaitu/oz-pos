import { useTheme } from './ThemeProvider';
import { useLocalization } from '@fluent/react';
import Tooltip from './Tooltip';

/**
 * A toggle button that switches between light and dark themes.
 * Shows a sun icon in dark mode (click to go light) and a moon
 * icon in light mode (click to go dark).
 */
export default function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();
  const { l10n } = useLocalization();
  const tooltipContent = l10n.getString('theme-toggle-label');

  // theme-toggle-aria is an attribute-only message (.aria-label), so we
  // access the FluentBundle directly to format its attribute with $mode.
  const bundle = l10n.getBundle('theme-toggle-aria');
  const msg = bundle?.getMessage('theme-toggle-aria');
  const ariaAttr = msg?.attributes?.['aria-label'];
  const ariaLabel = ariaAttr
    ? bundle!.formatPattern(ariaAttr, {
        mode: theme === 'light' ? 'dark' : 'light',
      }, [])
    : tooltipContent;

  return (
    <Tooltip content={tooltipContent} position="top">
      <button
        type="button"
        onClick={toggleTheme}
        className="theme-toggle"
        data-testid="theme-toggle"
        aria-label={ariaLabel}
      >
        <span className="sr-only">{tooltipContent}</span>
        {theme === 'light' ? (
          /* Moon icon (click to go dark) */
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
        ) : (
          /* Sun icon (click to go light) */
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <circle cx="12" cy="12" r="5" />
            <line x1="12" y1="1" x2="12" y2="3" />
            <line x1="12" y1="21" x2="12" y2="23" />
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
            <line x1="1" y1="12" x2="3" y2="12" />
            <line x1="21" y1="12" x2="23" y2="12" />
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
          </svg>
        )}
      </button>
    </Tooltip>
  );
}
