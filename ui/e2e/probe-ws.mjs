import { chromium } from '@playwright/test';

const CARD = 'button[data-testid="workspace-card"]';

async function probe() {
  const browser = await chromium.launch();
  const ctx = await browser.newContext({ locale: 'en-US', reducedMotion: 'reduce', viewport: { width: 1366, height: 768 } });
  const page = await ctx.newPage();
  await page.goto('http://localhost:1420/', { waitUntil: 'domcontentloaded' });
  try {
    await page.waitForSelector('.staff-login-screen', { timeout: 15000 });
    await page.locator('.staff-login-input').fill('admin');
    await page.locator('.staff-login-submit-btn').click();
    await page.locator('.staff-login-pad').waitFor({ state: 'visible', timeout: 10000 });
    for (const d of '9999') await page.locator('.staff-login-pad-key').filter({ hasText: d }).click();
    await page.waitForSelector('[data-testid="workspace-home"]', { timeout: 15000 });
  } catch (e) {
    console.log('LOGIN FAILED:', String(e).slice(0, 300));
    await browser.close();
    return;
  }

  const card = page.locator(CARD).filter({ hasText: 'Store POS' }).first();
  await card.waitFor({ state: 'visible', timeout: 5000 });

  const probe = await page.evaluate(() => {
    const el = document.querySelector('button[data-testid="workspace-card"]');
    if (!el) return { error: 'no card' };
    const rect1 = el.getBoundingClientRect();
    const a1 = getComputedStyle(el).animation;
    const t1 = getComputedStyle(el).transform;
    const p1 = el.parentElement ? getComputedStyle(el.parentElement).transform : null;
    return { rect: { x: rect1.x, y: rect1.y, w: rect1.width, h: rect1.height }, anim: a1, transform: t1, parentTransform: p1 };
  });
  console.log('PROBE1', JSON.stringify(probe, null, 1));
  await page.waitForTimeout(300);
  const probe2 = await page.evaluate(() => {
    const el = document.querySelector('button[data-testid="workspace-card"]');
    if (!el) return { error: 'no card' };
    const rect1 = el.getBoundingClientRect();
    return { x: rect1.x, y: rect1.y, w: rect1.width, h: rect1.height };
  });
  console.log('PROBE2', JSON.stringify(probe2));

  // Now hover the card and re-check — the failing actionability check happens
  // AFTER Playwright moves the mouse onto the element.
  await card.hover();
  await page.waitForTimeout(200);
  const probe3 = await page.evaluate(() => {
    const el = document.querySelector('button[data-testid="workspace-card"]');
    if (!el) return { error: 'no card' };
    const cs = getComputedStyle(el);
    const rect1 = el.getBoundingClientRect();
    const rm = matchMedia('(prefers-reduced-motion: reduce)').matches;
    const active = el.classList.contains('workspace-card--active');
    return { rm, active, anim: cs.animationName, animDuration: cs.animationDuration, rect: { x: rect1.x, y: rect1.y, w: rect1.width, h: rect1.height } };
  });
  console.log('PROBE_HOVER', JSON.stringify(probe3));
  await page.waitForTimeout(200);
  const probe4 = await page.evaluate(() => {
    const el = document.querySelector('button[data-testid="workspace-card"]');
    if (!el) return { error: 'no card' };
    const r = el.getBoundingClientRect();
    return { x: r.x, y: r.y, w: r.width, h: r.height };
  });
  console.log('PROBE_HOVER2', JSON.stringify(probe4));

  // Definitive test: attempt the actual click with default actionability.
  try {
    await card.click({ timeout: 8000 });
    console.log('CLICK_OK');
  } catch (e) {
    console.log('CLICK_FAIL:', String(e).slice(0, 400));
  }
  await browser.close();
}

probe().catch((e) => { console.error(e); process.exit(1); });
