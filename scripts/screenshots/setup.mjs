// The first-run page of a panel with no super admin yet, filled in but not sent.
import { open, sleep, go, both } from './lib.mjs';

const { FRESH_URL, FRESH_DATA } = process.env;
if (!FRESH_URL || !FRESH_DATA) throw new Error('Set FRESH_URL and FRESH_DATA to a panel with no super admin.');

const { browser, page } = await open({ width: 1200, height: 1300 });
await go(page, `${FRESH_URL}/register`);
for (const [name, value] of [
  ['setup_code', 'K7QM-2XRP-H9TC'],
  ['first_name', 'Ada'],
  ['last_name', 'Lovelace'],
  ['email', 'ada@example.com'],
  ['password', 'a-long-password'],
  ['confirm_password', 'a-long-password'],
]) {
  await page.type(`input[name="${name}"]`, value);
}
await page.click('body');
// The setup code's file as on a server installed as the guide says.
await page.evaluate(fresh => {
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    if (node.parentElement.closest('script, style, template')) continue;
    if (node.nodeValue.includes(fresh)) node.nodeValue = node.nodeValue.split(fresh).join('/var/lib/oxploy/data');
  }
}, FRESH_DATA);
await sleep(300);
// The form's column, from its eyebrow to its button, without the picture beside it.
const clip = await page.evaluate(() => {
  const eyebrow = [...document.querySelectorAll('p, span, div')].find(el => el.children.length === 0 && /first run/i.test(el.textContent.trim()));
  const top = eyebrow.getBoundingClientRect();
  const form = document.querySelector('form').getBoundingClientRect();
  const pad = 28;
  return { x: form.x - pad, y: top.y - pad, width: form.width + 2 * pad, height: form.bottom - top.y + 2 * pad };
});
await both(page, 'setup', clip);
await browser.close();
