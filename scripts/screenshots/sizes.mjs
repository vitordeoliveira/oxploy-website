// node sizes.mjs <screenshots folder> <pictures.rs>: writes each screenshot's size on the
// page, half its pixels, into the list of pictures.
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';

const [shots, pictures] = process.argv.slice(2);
let source = readFileSync(pictures, 'utf8');
for (const file of readdirSync(shots).filter(file => file.endsWith('-light.png')).sort()) {
  const name = file.slice(0, -'-light.png'.length);
  // A PNG's width and height are the 4-byte numbers at 16 and 20.
  const png = readFileSync(`${shots}/${file}`);
  const [width, height] = [png.readUInt32BE(16) / 2, png.readUInt32BE(20) / 2].map(Math.round);
  const entry = new RegExp(`picture!\\("${name}", \\d+ x \\d+\\)`);
  if (!entry.test(source)) {
    console.log(`${name} is not in ${pictures}: add picture!("${name}", ${width} x ${height})`);
    continue;
  }
  source = source.replace(entry, `picture!("${name}", ${width} x ${height})`);
  console.log(`${name}: ${width} x ${height}`);
}
writeFileSync(pictures, source);
