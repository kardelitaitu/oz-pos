/** Wrap an async action with console timing logs for dev observability. */
export async function withActionLog<T>(
  label: string,
  fn: () => Promise<T>,
): Promise<T> {
  const start = performance.now();
  console.log(`[action] ${label} → started`);
  try {
    const result = await fn();
    const elapsed = Math.round(performance.now() - start);
    console.log(`[action] ${label} → succeeded (${elapsed}ms)`);
    return result;
  } catch (err) {
    const elapsed = Math.round(performance.now() - start);
    console.log(`[action] ${label} → failed (${elapsed}ms)`, err);
    throw err;
  }
}
