/**
 * rehype-mermaid-class — bridge Astro 7 shiki output to rehype-mermaid.
 *
 * Astro's shiki integration marks fenced code blocks with
 * `data-language="mermaid"` on the <pre> instead of the
 * `class="language-mermaid"` that rehype-mermaid matches (shiki runs before
 * user rehype plugins). This pre-pass adds `class="mermaid"` to such <pre>
 * elements so rehypeMermaid — next in the processor — picks them up and
 * replaces them with inline SVG.
 *
 * Dependency-free on purpose: it only walks the hast tree and reads the
 * attribute shiki already wrote.
 */

function walk(node) {
  if (!node || typeof node !== 'object') return;

  if (node.type === 'element' && node.tagName === 'pre') {
    const props = node.properties ?? {};
    const lang = props.dataLanguage ?? props['data-language'];
    if (lang === 'mermaid') {
      props.className = [
        ...(Array.isArray(props.className) ? props.className : []),
        'mermaid',
      ];
    }
  }

  const children = node.children;
  if (Array.isArray(children)) {
    for (const child of children) walk(child);
  }
}

export default function rehypeMermaidClass() {
  return (tree) => walk(tree);
}
