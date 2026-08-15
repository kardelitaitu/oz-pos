/**
 * rehype-callouts — GitBook-style labeled blockquotes become themed callouts.
 *
 * Markdown like this:
 *
 *   > **Warning:** Your stock will not sync while offline.
 *
 * gets the classes `callout callout-warning` on the <blockquote>, so CSS can
 * give each kind a distinct left border and tint (see global.css). The
 * default accent styling applies to any blockquote without a recognized
 * label — plain `> text` or `> **Note:** …` both work.
 *
 * Dependency-free on purpose: it walks the hast tree by hand, so it only
 * relies on the shape Astro's markdown pipeline already produces.
 */

const KINDS = {
  note: 'note',
  info: 'info',
  tip: 'tip',
  warning: 'warning',
  caution: 'warning',
  danger: 'warning',
};

function walk(node) {
  if (!node || typeof node !== 'object') return;

  const children = node.children;
  if (Array.isArray(children)) {
    for (const child of children) walk(child);
  }

  if (node.type !== 'element' || node.tagName !== 'blockquote') return;

  // First child element must be a paragraph whose first element is a strong
  // label like "Warning:".
  const first = children?.find((c) => c.type === 'element');
  if (!first || first.tagName !== 'p') return;
  const lead = first.children?.find((c) => c.type === 'element');
  if (!lead || lead.tagName !== 'strong') return;

  const text = String(lead.children?.[0]?.value ?? '').trim();
  const kind = KINDS[text.replace(/:$/, '').toLowerCase()];
  if (!kind) return;

  const props = (node.properties ??= {});
  props.className = [
    ...(Array.isArray(props.className) ? props.className : []),
    'callout',
    `callout-${kind}`,
  ];
}

export default function rehypeCallouts() {
  return (tree) => walk(tree);
}
