declare module 'jest-axe' {
  import type { RunOptions, AxeResults } from 'axe-core';

  /** Custom Jest/Vitest matcher for axe-core results. */
  export const toHaveNoViolations: Record<
    string,
    (results: AxeResults) => { pass: boolean; message(): string }
  >;

  export function configureAxe(
    options?: {
      globalOptions?: {
        rules?: Array<{ id: string; enabled: boolean }>;
        [key: string]: unknown;
      };
    } & Omit<RunOptions, 'rules'>,
  ): (html: string | Element, additionalOptions?: RunOptions) => Promise<AxeResults>;

  export function axe(
    html: string | Element,
    additionalOptions?: RunOptions,
  ): Promise<AxeResults>;
}
