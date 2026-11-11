// Takes the screenshots of the seeded demo panel: node capture.mjs [name]...
//
// The panel shows what the machine it runs on is: its addresses, the demo's folders, its
// environment, its system. Each screenshot shows a server installed as the docs say
// instead, so none of this machine's leaves it.
import { open, BASE, clickText, sleep, go, submit, nth, dialog, field, both } from './lib.mjs';

const only = process.argv.slice(2);
const { browser, page } = await open();
const service = id => `${BASE}/dashboard/projects/1/services/${id}`;
const DEMO_DATA = process.env.DEMO_DATA;
const PANEL = new URL(BASE).host;
if (!DEMO_DATA) throw new Error('Set DEMO_DATA to the demo panel OXPLOY_DATA_DIR.');

/** Puts what a server installed as the docs say shows in place of this machine's. */
async function scrub() {
  const mcpPage = page.url().includes('/dashboard/mcp');
  await page.evaluate((demo, panel, mcpPage) => {
    const replace = text => {
      // The MCP page names the panel's own address, which a server has at its domain.
      if (mcpPage) text = text.split(`http://${panel}/mcp`).join('https://panel.example.com/mcp');
      return text
        .split(demo).join('/var/lib/oxploy/data')
        .split(panel).join('127.0.0.1:3000')
        .replace(/\b(?:aarch64|x86_64) macOS\b/g, 'x86_64 Linux')
        // The machine's IPv6 addresses, one or a list of them.
        .replace(/\b(?:[0-9a-f]{1,4}:){3,7}[0-9a-f]{1,4}\b(?:,\s*(?:[0-9a-f]{1,4}:){3,7}[0-9a-f]{1,4}\b)*/gi, '203.0.113.7');
    };
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      // The runtime keeps its state in scripts, which must stay as they are.
      if (node.parentElement && node.parentElement.closest('script, style, template')) continue;
      const next = replace(node.nodeValue);
      if (next !== node.nodeValue) node.nodeValue = next;
    }
    for (const input of document.querySelectorAll('input, textarea')) {
      const next = replace(input.value || '');
      if (next !== input.value) input.value = next;
    }
    // The demo runs no Caddy, which a server does.
    for (const el of document.querySelectorAll('div')) {
      if (/^Caddy is not running/.test((el.innerText || '').trim()) && el.className.includes('border')) el.style.display = 'none';
    }
  }, DEMO_DATA, PANEL, mcpPage);
}

const tall = height => page.setViewport({ width: 1024, height, deviceScaleFactor: 2 });

/** Scrubs the page, then takes `name` of what `target` finds once scrubbed. */
async function shoot(name, target = async () => undefined) {
  await scrub();
  await sleep(200);
  await both(page, name, await target());
}

/** The card whose title is `title`. */
async function card(title) {
  const marked = await page.evaluate(title => {
    for (const heading of document.querySelectorAll('h1, h2, h3, h4, div, span')) {
      if (heading.children.length > 0 || heading.textContent.trim() !== title) continue;
      if (heading.getBoundingClientRect().width === 0) continue;
      let el = heading;
      while (el && !(typeof el.className === 'string' && el.className.includes('rounded-xl') && el.className.includes('border'))) el = el.parentElement;
      if (el) {
        el.setAttribute('data-shot', title);
        return true;
      }
    }
    return false;
  }, title);
  if (!marked) throw new Error('no card titled ' + title);
  return page.$(`[data-shot="${title}"]`);
}

/** The cards from the one titled `first` to the one titled `last`, as one clip. */
async function cards(first, last) {
  const a = await (await card(first)).boundingBox();
  const b = await (await card(last)).boundingBox();
  return { x: a.x - 1, y: a.y - 1, width: Math.max(a.width, b.width) + 2, height: b.y + b.height - a.y + 2 };
}

/** Hides a line that only a demo without GitHub shows. */
async function hide(text) {
  await page.evaluate(text => {
    for (const el of document.querySelectorAll('p, span, div')) {
      if (el.children.length === 0 && el.textContent.trim() === text) el.style.display = 'none';
    }
  }, text);
}

/** The page's content beside the sidebar. */
async function main() {
  const handle = await page.evaluateHandle(() => document.querySelector('main') || document.querySelector('[data-sidebar=inset]'));
  return handle.asElement();
}

const shots = {
  async project() {
    await tall(760);
    await go(page, `${BASE}/dashboard/projects/1`);
    await shoot('project');
  },
  async 'new-service'() {
    await tall(900);
    await go(page, `${BASE}/dashboard/projects/1`);
    await clickText(page, 'New service');
    await dialog(page);
    await page.type(field('name'), 'shop-web');
    await page.type(field('description'), 'The storefront.');
    await shoot('new-service', async () => await dialog(page));
  },
  async 'external-mcp'() {
    await tall(900);
    await go(page, `${BASE}/dashboard/projects/1`);
    await clickText(page, 'New service');
    await dialog(page);
    await page.click('[data-shot="dialog"] input[name="kind"][value="external_mcp"]');
    await sleep(400);
    await page.type(field('name'), 'billing');
    await page.type(field('url'), 'https://billing.example.com/mcp');
    await shoot('external-mcp', async () => await dialog(page));
  },
  async 'build-cargo'() {
    await tall(1700);
    await go(page, service(1));
    const form = 'form[action$="/general/build"]';
    for (const [name, value] of [['rust_version', '1.85'], ['binary', 'shop-api'], ['features', 'postgres, tls']]) {
      await page.$eval(`${form} input[name="${name}"]`, el => { el.value = ''; });
      await page.type(`${form} input[name="${name}"]`, value);
    }
    await submit(page, 'Save build');
    await shoot('build-cargo', async () => await card('Build'));
  },
  async 'build-release'() {
    await tall(1700);
    await go(page, service(1) + '?build=release');
    await page.click('form[action$="/general/release"] input[name="file"]');
    await page.keyboard.type('shop-api-*-x86_64-linux.tar.gz', { delay: 20 });
    await sleep(600);
    await page.click('form[action$="/general/release"] input[name="automatic"]');
    await sleep(400);
    await shoot('build-release', async () => await card('Build'));
  },
  async 'deploy-card'() {
    await tall(900);
    await go(page, service(2));
    await clickText(page, 'Restart');
    await sleep(5000);
    await hide('Save a source below to deploy.');
    await shoot('deploy-card', async () => await card('Deploy'));
  },
  async environment() {
    await tall(2600);
    await go(page, service(2) + '/environment');
    // What a server installed as the guide says gives its apps, not this machine.
    await page.evaluate(() => {
      const values = {
        PATH: '/var/lib/oxploy/.cargo/bin:/usr/local/bin:/usr/bin:/bin',
        HOME: '/var/lib/oxploy',
        USER: 'oxploy',
        LOGNAME: 'oxploy',
        LANG: 'C.UTF-8',
        LC_ALL: null,
        TMPDIR: null,
      };
      for (const el of document.querySelectorAll('div, span, dt, td, code')) {
        const key = el.textContent.trim();
        if (el.children.length !== 0 || !(key in values)) continue;
        const row = el.parentElement;
        if (values[key] === null) {
          row.style.display = 'none';
          continue;
        }
        const value = [...row.querySelectorAll('*')].find(other => other !== el && other.children.length === 0 && other.textContent.trim() !== key);
        if (value) value.textContent = values[key];
      }
    });
    await shoot('environment', async () => await card('What the app gets'));
  },
  async domains() {
    await tall(900);
    await go(page, service(2) + '/domains', 2500);
    await shoot('domains', async () => await card('Domains'));
  },
  async storage() {
    await tall(1500);
    await go(page, service(2) + '/storage');
    await shoot('storage', async () => await cards('Storage', 'Backups'));
  },
  async 'monitoring-app'() {
    await tall(1200);
    await go(page, service(2) + '/monitoring', 3000);
    await shoot('monitoring-app', async () => await main());
  },
  async 'monitoring-server'() {
    await tall(1200);
    await go(page, `${BASE}/dashboard/monitoring`, 3000);
    // The four charts, without the machine's name, system and processor above them.
    await shoot('monitoring-server', async () => page.evaluate(() => {
      const box = title => {
        const walker = document.createTreeWalker(document.querySelector('main') || document.body, NodeFilter.SHOW_TEXT);
        let el = null;
        for (let node = walker.nextNode(); node && !el; node = walker.nextNode()) {
          if (node.nodeValue.trim() === title && node.parentElement.getBoundingClientRect().width > 0) el = node.parentElement;
        }
        // Up to the card, whose parent is the grid of two columns.
        const width = node => node.getBoundingClientRect().width;
        while (el.parentElement && !(width(el) > 300 && width(el.parentElement) >= width(el) * 1.6)) el = el.parentElement;
        return el.getBoundingClientRect();
      };
      const memory = box('Memory');
      const disk = box('Disk I/O');
      return { x: disk.x - 1, y: memory.y - 1, width: memory.right - disk.x + 2, height: disk.bottom - memory.y + 2 };
    }));
  },
  async 'app-mcp'() {
    await tall(1300);
    await go(page, service(2) + '/mcp', 1500);
    await shoot('app-mcp', async () => await cards('MCP server', 'Settings'));
  },
  async 'app-mcp-tools'() {
    await tall(2000);
    await go(page, service(2) + '/mcp', 1500);
    await shoot('app-mcp-tools', async () => await card('Tools'));
  },
  async 'mcp-server-token'() {
    await tall(900);
    await go(page, `${BASE}/dashboard/mcp`);
    await nth(page, 'New token', 0);
    await dialog(page);
    await page.type(field('name'), 'Claude Code');
    await shoot('mcp-server-token', async () => await dialog(page));
  },
  async 'mcp-org-token'() {
    await tall(1100);
    await go(page, `${BASE}/dashboard/mcp`);
    await nth(page, 'New token', 1);
    await dialog(page);
    await page.type(field('name'), 'Support bot');
    for (const tool of ['search_docs', 'read_doc']) await page.click(`[data-shot="dialog"] input[name="tool"][value$=":${tool}"]`);
    await page.click('[data-shot="dialog"] input[name="for_apps"]');
    await sleep(400);
    await shoot('mcp-org-token', async () => await dialog(page));
  },
  async 'mcp-page'() {
    await tall(1400);
    await go(page, `${BASE}/dashboard/mcp`);
    // The page down to its last card.
    await shoot('mcp-page', async () => page.evaluate(() => {
      const main = document.querySelector('main') || document.querySelector('[data-sidebar=inset]');
      const top = main.getBoundingClientRect();
      const last = [...main.querySelectorAll('[class*="rounded-xl"][class*="border"]')].map(el => el.getBoundingClientRect().bottom).reduce((a, b) => Math.max(a, b), 0);
      return { x: top.x, y: top.y, width: top.width, height: last - top.y + 24 };
    }));
  },
  async settings() {
    await tall(1600);
    await go(page, `${BASE}/dashboard/settings`, 2000);
    await shoot('settings', async () => await cards('Panel domain', 'Time zone'));
    await shoot('settings-version', async () => await card('Version'));
  },
};

let failed = 0;
for (const [name, take] of Object.entries(shots)) {
  if (only.length && !only.includes(name)) continue;
  try {
    await take();
    console.log('took', name);
  } catch (error) {
    failed += 1;
    console.log('FAILED', name, error.message);
  }
}
await browser.close();
process.exit(failed ? 1 : 0);
