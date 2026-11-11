// What the scripts share: the browser, and how they find their way in the panel.
import { existsSync } from 'node:fs';
import puppeteer from 'puppeteer-core';

/** The demo panel, as run.sh starts it. */
export const BASE = process.env.PANEL_URL || 'http://127.0.0.1:3950';

/** Where the screenshots go, as PNG, before run.sh makes them WebP. */
export const OUT = process.env.SHOTS_DIR || new URL('./out/', import.meta.url).pathname;

/** A Chromium the machine has: BROWSER, or else the first one found. */
function browserPath() {
  const candidates = [
    process.env.BROWSER,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Brave Browser.app/Contents/MacOS/Brave Browser',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
    '/usr/bin/google-chrome',
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
  ];
  const found = candidates.find(path => path && existsSync(path));
  if (!found) throw new Error('No Chromium found: set BROWSER to one.');
  return found;
}

/**
 * A browser with one page, 1024 pixels wide at twice the scale, so a screenshot's text
 * stays legible in the docs' column.
 */
export async function open({ width = 1024, height = 900, scale = 2, profile = process.env.PROFILE } = {}) {
  const browser = await puppeteer.launch({
    executablePath: browserPath(),
    headless: true,
    userDataDir: profile,
    args: ['--no-first-run', '--no-default-browser-check', '--hide-scrollbars'],
  });
  const page = await browser.newPage();
  await page.setViewport({ width, height, deviceScaleFactor: scale });
  return { browser, page };
}

export const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));

/** Opens `url`, then waits for the page's runtime to settle. */
export async function go(page, url, wait = 800) {
  await page.goto(url, { waitUntil: 'networkidle0' });
  await sleep(wait);
}

/** Clicks the visible element whose text is exactly `text`. */
export async function clickText(page, text, selector = 'button, a, [role=menuitem], [role=tab], [role=option], label') {
  for (const handle of await page.$$(selector)) {
    const found = await handle.evaluate((el, text) => {
      const box = el.getBoundingClientRect();
      return box.width > 0 && box.height > 0 && (el.innerText || el.textContent || '').trim() === text;
    }, text);
    if (found) {
      await handle.click();
      return handle;
    }
  }
  throw new Error('no clickable with text: ' + text);
}

/** Clicks the button with `text`, and waits for the page the form sends the browser to. */
export async function submit(page, text) {
  await Promise.all([page.waitForNavigation({ waitUntil: 'networkidle0' }).catch(() => {}), clickText(page, text)]);
  await sleep(600);
}

/** Clicks the `index`th visible button whose text is `text`, such as the second New token. */
export async function nth(page, text, index) {
  const found = [];
  for (const button of await page.$$('button')) {
    if (await button.evaluate((el, text) => el.innerText.trim() === text && el.getBoundingClientRect().width > 0, text)) found.push(button);
  }
  await found[index].click();
  // A dialog opens once the runtime has hydrated its signal.
  await sleep(900);
}

/** The open dialog's panel, not the overlay behind it, marked so `field` finds its fields. */
export async function dialog(page) {
  await page.waitForSelector('dialog[open]', { timeout: 5000 });
  await sleep(400);
  await page.evaluate(() => {
    const root = document.querySelector('dialog[open]:not([data-sidebar])')
      || [...document.querySelectorAll('dialog[open], [role=dialog]')].find(el => el.getBoundingClientRect().width > 300);
    const panel = [...root.querySelectorAll('*'), root].find(el => {
      const box = el.getBoundingClientRect();
      return box.width > 200 && box.width < innerWidth - 100 && box.height > 100;
    });
    panel.setAttribute('data-shot', 'dialog');
  });
  return page.$('[data-shot="dialog"]');
}

/** The field `name` of the open dialog. */
export const field = name => `[data-shot="dialog"] [name="${name}"]`;

/**
 * Saves <name>-light.png and <name>-dark.png of `target`: an element handle, a clip
 * { x, y, width, height }, or nothing for the viewport.
 */
export async function both(page, name, target) {
  for (const theme of ['light', 'dark']) {
    await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: theme }]);
    await sleep(400);
    const path = `${OUT}/${name}-${theme}.png`;
    // The panel scrolls inside its own container, which a capture beyond the viewport
    // misplaces, so the viewport is made tall enough instead.
    if (target && target.boundingBox) {
      const box = await target.boundingBox();
      await page.screenshot({ path, clip: { x: box.x, y: box.y, width: box.width, height: box.height }, captureBeyondViewport: false });
    } else if (target && target.width) {
      await page.screenshot({ path, clip: target, captureBeyondViewport: false });
    } else {
      await page.screenshot({ path });
    }
  }
  await page.emulateMediaFeatures([{ name: 'prefers-color-scheme', value: 'light' }]);
}
