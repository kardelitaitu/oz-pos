import { defineCollection } from 'astro:content';
import { glob } from 'astro/loaders';
import { z } from 'zod';

/**
 * Documentation collection (GitBook-style, website-plan §docs).
 * Files: src/content/docs/*.md — category + order drive the sidebar.
 */
const docs = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/docs' }),
  schema: z.object({
    title: z.string(),
    description: z.string().optional(),
    category: z.enum(['gettingStarted', 'guides', 'reference']),
    order: z.number().default(0),
    updated: z.string().optional(),
  }),
});

/**
 * Legal collection (Privacy Policy / Terms of Service).
 * Files: src/content/legal/<locale>/{privacy,terms}.md.
 * The page <h1> comes from i18n (legal.privacyTitle / legal.termsTitle);
 * frontmatter carries the last-updated date shown under it.
 */
const legal = defineCollection({
  loader: glob({ pattern: '**/*.md', base: './src/content/legal' }),
  schema: z.object({
    title: z.string(),
    updated: z.string().optional(),
  }),
});

export const collections = { docs, legal };
