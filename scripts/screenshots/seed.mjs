// Fills the demo panel with example data, through its pages, as someone would.
//
//   node seed.mjs start <setup code>  the super admin, the shop project and its services
//   node seed.mjs app                 once the docs app runs: its MCP server, variables,
//                                     domains, database, tools, and the MCP tokens
import { open, BASE, clickText, sleep, go, submit, nth, dialog, field } from './lib.mjs';

const [phase, code] = process.argv.slice(2);
const { browser, page } = await open({ scale: 1 });
const project = `${BASE}/dashboard/projects/1`;
const service = id => `${project}/services/${id}`;

/** Empties the text field `selector`, then types `value` in it. */
async function set(selector, value) {
  await page.$eval(selector, el => { el.value = ''; });
  if (value) await page.type(selector, value);
}

/** Clicks the button with `text` inside `form`, and waits for where it sends the browser. */
async function send(form, text) {
  for (const button of await page.$$(`${form} button`)) {
    if ((await button.evaluate(el => el.innerText.trim())) === text) {
      await Promise.all([page.waitForNavigation({ waitUntil: 'networkidle0' }).catch(() => {}), button.click()]);
      await sleep(500);
      return;
    }
  }
  throw new Error(`no button ${text} in ${form}`);
}

/** Sends the page's form, such as the sign-in. */
async function sendForm() {
  await Promise.all([page.waitForNavigation({ waitUntil: 'networkidle0' }), page.click('button[type="submit"]')]);
  await sleep(500);
}

if (phase === 'start') {
  await go(page, `${BASE}/register`);
  for (const [name, value] of [
    ['setup_code', code],
    ['first_name', 'Ada'],
    ['last_name', 'Lovelace'],
    ['email', 'ada@example.com'],
    ['password', 'demo-password-1'],
    ['confirm_password', 'demo-password-1'],
  ]) {
    await page.type(`input[name="${name}"]`, value);
  }
  await sendForm();
  if (page.url().includes('/login')) {
    await page.type('input[name="email"]', 'ada@example.com');
    await page.type('input[name="password"]', 'demo-password-1');
    await sendForm();
  }

  await go(page, `${BASE}/dashboard/projects`);
  await clickText(page, 'New project');
  await sleep(400);
  await page.type('form[action="/dashboard/projects"] input[name="name"]', 'shop');
  await page.type('form[action="/dashboard/projects"] textarea[name="description"]', 'The shop: its API, its site and its docs.');
  await submit(page, 'Create project');

  for (const [name, description] of [
    ['shop-api', 'The orders and payments API.'],
    ['docs', "The shop's documentation, with its MCP server."],
  ]) {
    await go(page, project);
    await clickText(page, 'New service');
    await dialog(page);
    await page.type(field('name'), name);
    await page.type(field('description'), description);
    await submit(page, 'Create service');
  }
  console.log('seeded the project and its services');
}

if (phase === 'app') {
  // The app's MCP server at /admin/mcp, where the docs site serves it to oxploy.
  await go(page, service(2) + '/mcp');
  const settings = 'form[action$="/mcp-server/settings"]';
  if (!(await page.$eval(`${settings} input[name="on"]`, el => el.checked))) await page.click(`${settings} input[name="on"]`);
  await set(`${settings} input[name="path"]`, '/admin/mcp');
  await send(settings, 'Save');
  // It gets its token when it starts.
  await go(page, service(2));
  await clickText(page, 'Restart');
  await sleep(6000);

  await go(page, service(2) + '/environment');
  await set('textarea[name="variables"]', 'RUST_LOG=info\nADMIN_USERNAME=ada\nADMIN_PASSWORD=correct-horse-battery-staple\nPUBLIC_URL=https://docs.localhost');
  await submit(page, 'Save variables');

  for (const [host, path] of [['docs.localhost', ''], ['shop.localhost', '/docs']]) {
    await go(page, service(2) + '/domains');
    await clickText(page, 'Add domain');
    await sleep(500);
    const form = 'form[action$="/hosts/add-domain"]';
    await page.type(`${form} input[name="host"]`, host);
    if (path) await page.type(`${form} input[name="path"]`, path);
    await send(form, 'Add domain');
  }

  // The site's own database, which oxploy takes over, and a backup of it.
  await go(page, service(2) + '/storage');
  await clickText(page, 'Manage');
  await sleep(500);
  await set('form[action$="/stored/databases"] input[name="link"]', '');
  await send('form[action$="/stored/databases"]', 'Manage');
  await go(page, service(2) + '/storage');
  await submit(page, 'Back up');

  // Three of the app's four tools, so the tab shows one off.
  await go(page, service(2) + '/mcp', 1500);
  for (const toggle of (await page.$$('input[type=checkbox]:not([name])')).slice(0, 3)) {
    await toggle.click();
    await sleep(700);
  }

  // A server token, and one of the organization for a bot that runs here as an app.
  await go(page, `${BASE}/dashboard/mcp`);
  await nth(page, 'New token', 0);
  await dialog(page);
  await page.type(field('name'), 'Claude Local');
  await submit(page, 'Create token');
  await go(page, `${BASE}/dashboard/mcp`);
  await nth(page, 'New token', 1);
  await dialog(page);
  await page.type(field('name'), 'Telegram bot');
  for (const tool of ['search_docs', 'read_doc']) await page.click(`[data-shot="dialog"] input[name="tool"][value$=":${tool}"]`);
  await page.click('[data-shot="dialog"] input[name="for_apps"]');
  await submit(page, 'Create token');
  console.log('seeded the app');
}

await browser.close();
