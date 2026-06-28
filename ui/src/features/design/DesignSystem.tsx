import { Localized } from '@/components/Localized';
import ThemeToggle from '@/components/ThemeToggle';

/**
 * Design System showcase — visual reference of every token category.
 *
 * Renders colour swatches, typography scale, spacing, shadows,
 * semantic buttons, and form elements so developers can inspect
 * the active theme at a glance.
 *
 * This screen is intended for development only and can be gated
 * behind a feature flag or stripped in production builds.
 */
export default function DesignSystem() {
  return (
    <div className="ds-page">
      <header className="ds-header">
        <h1>
          <Localized id="ds-title">
            <span>Design System</span>
          </Localized>
        </h1>
        <ThemeToggle />
      </header>

      <div className="ds-layout">
        {/* ── Colors ────────────────────────────── */}
        <Section title="Colors">
          <SwatchRow label="Neutral">
            {([50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950] as const).map((n) => (
              <Swatch key={n} colour={`var(--neutral-${n})`} label={`${n}`} />
            ))}
          </SwatchRow>

          <SwatchRow label="Primary (Emerald)">
            {([50, 100, 200, 300, 400, 500, 600, 700, 800, 900, 950] as const).map((n) => (
              <Swatch key={n} colour={`var(--primary-${n})`} label={`${n}`} />
            ))}
          </SwatchRow>

          <SwatchRow label="Semantic">
            <Swatch colour="var(--color-success)" label="Success" />
            <Swatch colour="var(--color-warning)" label="Warning" />
            <Swatch colour="var(--color-danger)" label="Danger" />
            <Swatch colour="var(--color-info)" label="Info" />
          </SwatchRow>

          <SwatchRow label="Backgrounds">
            <Swatch colour="var(--color-bg)" label="Base" />
            <Swatch colour="var(--color-bg-elevated)" label="Elevated" />
            <Swatch colour="var(--color-bg-surface)" label="Surface" />
            <Swatch colour="var(--color-bg-input)" label="Input" />
          </SwatchRow>

          <SwatchRow label="Text">
            <Swatch colour="var(--color-fg-primary)" label="Primary" />
            <Swatch colour="var(--color-fg)" label="Body" />
            <Swatch colour="var(--color-fg-secondary)" label="Secondary" />
            <Swatch colour="var(--color-fg-tertiary)" label="Tertiary" />
            <Swatch colour="var(--color-fg-disabled)" label="Disabled" />
          </SwatchRow>

          <SwatchRow label="Borders">
            <Swatch colour="var(--color-border)" label="Default" />
            <Swatch colour="var(--color-border-hover)" label="Hover" />
            <Swatch colour="var(--color-border-strong)" label="Strong" />
            <Swatch colour="var(--color-border-focus)" label="Focus" />
          </SwatchRow>

          <SwatchRow label="Accent">
            <Swatch colour="var(--color-accent)" label="Default" />
            <Swatch colour="var(--color-accent-hover)" label="Hover" />
            <Swatch colour="var(--color-accent-active)" label="Active" />
            <Swatch colour="var(--color-accent-subtle)" label="Subtle" />
          </SwatchRow>
        </Section>

        {/* ── Typography ────────────────────────── */}
        <Section title="Typography">
          <div className="ds-type-grid">
            <TypeSample tag="h1" token="text-2xl" value="1.5rem">
              The quick brown fox jumps over the lazy dog
            </TypeSample>
            <TypeSample tag="h2" token="text-xl" value="1.25rem">
              The quick brown fox jumps over the lazy dog
            </TypeSample>
            <TypeSample tag="h3" token="text-lg" value="1.125rem">
              The quick brown fox jumps over the lazy dog
            </TypeSample>
            <TypeSample token="text-base" value="0.875rem">
              The quick brown fox jumps over the lazy dog — 14px body
            </TypeSample>
            <TypeSample token="text-sm" value="0.75rem">
              The quick brown fox jumps over the lazy dog — 12px small
            </TypeSample>
            <TypeSample token="text-xs" value="0.625rem">
              The quick brown fox jumps over the lazy dog — 10px extra small
            </TypeSample>
          </div>

          <div className="ds-type-styles">
            <p><strong>Weight:</strong> Normal &middot; <span style={{ fontWeight: 500 }}>Medium</span> &middot; <span style={{ fontWeight: 600 }}>Semibold</span> &middot; <span style={{ fontWeight: 700 }}>Bold</span></p>
            <p><strong>Mono:</strong> <code>const x: string = &quot;hello&quot;;</code></p>
            <p><strong>Link:</strong> <a href="https://example.com">Clickable link</a></p>
          </div>
        </Section>

        {/* ── Spacing ──────────────────────────── */}
        <Section title="Spacing">
          <div className="ds-space-grid">
            {([0, 0.5, 1, 1.5, 2, 2.5, 3, 3.5, 4, 5, 6, 8, 10, 12, 16, 20, 24] as const).map((s) => {
              const name = s === Math.floor(s) ? `${s}` : `${s}`.replace('.', '_');
              return (
                <div key={name} className="ds-space-item">
                  <div
                    className="ds-space-bar"
                    style={{ width: `var(--space-${name})` }}
                  />
                  <span className="ds-space-label">
                    --space-{name} <br />({s > 0 ? `${s}rem` : '0'})
                  </span>
                </div>
              );
            })}
          </div>
        </Section>

        {/* ── Shadows ──────────────────────────── */}
        <Section title="Shadows">
          <div className="ds-shadow-grid">
            {(['xs', 'sm', 'md', 'lg', 'xl', '2xl', 'inner'] as const).map((s) => (
              <div
                key={s}
                className="ds-shadow-card"
                style={{ boxShadow: `var(--shadow-${s})` }}
              >
                <code>--shadow-{s}</code>
              </div>
            ))}
          </div>
        </Section>

        {/* ── Border radius ────────────────────── */}
        <Section title="Border Radius">
          <div className="ds-radius-grid">
            {(['none', 'sm', 'md', 'lg', 'xl', '2xl', '3xl', 'full'] as const).map((r) => (
              <div
                key={r}
                className="ds-radius-box"
                style={{ borderRadius: `var(--radius-${r})` }}
              >
                <span>{r}</span>
              </div>
            ))}
          </div>
        </Section>

        {/* ── Buttons ──────────────────────────── */}
        <Section title="Buttons">
          <div className="ds-button-row">
            <button className="btn btn--primary">Primary</button>
            <button className="btn btn--secondary">Secondary</button>
            <button className="btn btn--danger">Danger</button>
            <button className="btn btn--ghost">Ghost</button>
            <button className="btn btn--primary" disabled>Disabled</button>
          </div>
        </Section>

        {/* ── Form elements ────────────────────── */}
        <Section title="Form Elements">
          <div className="ds-form-grid">
            <label className="ds-field">
              <span>Text input</span>
              <input type="text" placeholder="Placeholder text" />
            </label>
            <label className="ds-field">
              <span>Select</span>
              <select>
                <option>Option 1</option>
                <option>Option 2</option>
                <option>Option 3</option>
              </select>
            </label>
            <label className="ds-field">
              <span>Textarea</span>
              <textarea rows={3} placeholder="Write something…" />
            </label>
          </div>
        </Section>
      </div>
    </div>
  );
}

// ── Internal sub-components ────────────────────────────────────────

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="ds-section">
      <h2 className="ds-section-title">{title}</h2>
      {children}
    </section>
  );
}

function Swatch({ colour, label }: { colour: string; label: string }) {
  return (
    <div className="ds-swatch">
      <div className="ds-swatch-box" style={{ background: colour }} />
      <span className="ds-swatch-label">{label}</span>
    </div>
  );
}

function SwatchRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="ds-swatch-row">
      <span className="ds-swatch-row-label">{label}</span>
      <div className="ds-swatch-group">{children}</div>
    </div>
  );
}

function TypeSample({
  tag,
  token,
  value,
  children,
}: {
  tag?: 'h1' | 'h2' | 'h3';
  token: string;
  value: string;
  children: React.ReactNode;
}) {
  const style = {
    fontSize: `var(--${token})`,
    margin: 0,
  } as React.CSSProperties;

  const Tag = tag ?? 'p';
  return (
    <div className="ds-type-sample">
      <Tag style={style}>{children}</Tag>
      <span className="ds-type-meta">
        {token} / {value}
      </span>
    </div>
  );
}
