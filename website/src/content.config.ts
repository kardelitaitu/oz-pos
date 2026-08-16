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

export const collections = { docs };
