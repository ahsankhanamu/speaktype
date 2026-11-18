import { copyFileSync, mkdirSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const out = process.argv[process.argv.indexOf('--out') + 1];
if (!out || out.startsWith('--')) throw new Error('usage: npm run screens -- --out <dir>');

const source = fileURLToPath(new URL('../tests/visual/__screenshots__', import.meta.url));
mkdirSync(resolve(out), { recursive: true });
for (const file of readdirSync(source).filter((name) => name.endsWith('.png'))) copyFileSync(join(source, file), join(resolve(out), file));
