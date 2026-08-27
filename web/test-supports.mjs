import puppeteer from 'puppeteer';
import { createServer } from 'vite';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, '..');

async function main() {
  const server = await createServer({
    root: __dirname,
    server: { port: 4173, strictPort: true },
  });
  await server.listen();

  const browser = await puppeteer.launch({ headless: true, args: ['--no-sandbox'] });
  const page = await browser.newPage();
  await page.setViewport({ width: 1280, height: 900 });

  page.on('console', msg => console.log('BROWSER:', msg.text()));
  page.on('pageerror', err => console.log('PAGE ERROR:', err.message));

  await page.goto('http://localhost:4173', { waitUntil: 'networkidle0' });
  const fileInput = await page.$('input[type="file"]');
  await fileInput.uploadFile(path.resolve(root, 'resources/Skulled_Wurm_Bird_WOBase.stl'));

  console.log('Waiting 15s for pipeline...');
  await new Promise(r => setTimeout(r, 15000));

  // Dump status element
  const status = await page.evaluate(() => {
    const el = document.querySelector('#status');
    return el ? el.textContent : 'NO #status element';
  });
  console.log('Status:', status);

  // Check if Generate button exists and is enabled
  const btnInfo = await page.evaluate(() => {
    const buttons = document.querySelectorAll('button');
    const results = [];
    for (const btn of buttons) {
      results.push({
        text: btn.textContent?.trim(),
        disabled: btn.disabled,
        display: getComputedStyle(btn).display,
      });
    }
    return results;
  });
  console.log('Buttons:', JSON.stringify(btnInfo, null, 2));

  // Click Generate Supports
  await page.evaluate(() => {
    const buttons = document.querySelectorAll('button');
    for (const btn of buttons) {
      if (btn.textContent?.includes('Generate')) { btn.click(); break; }
    }
  });

  console.log('Waiting 20s for supports...');
  await new Promise(r => setTimeout(r, 20000));

  const status2 = await page.evaluate(() => {
    const el = document.querySelector('#status');
    return el ? el.textContent : 'NO #status element';
  });
  console.log('Status after generate:', status2);

  await page.screenshot({ path: path.resolve(root, 'web/support-test.png'), fullPage: false });
  console.log('Screenshot saved');

  await browser.close();
  await server.close();
}

main().catch(e => { console.error(e); process.exit(1); });
