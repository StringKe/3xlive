/**
 * 生成 og-image.png（1200×630）和 apple-touch-icon.png（180×180）。
 * 流程：SVG → sharp 光栅化。
 *
 * 用法：
 *   node scripts/gen-og-image.mjs
 */

import sharp from "sharp";
import { mkdirSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT = resolve(__dirname, "../public");
mkdirSync(OUT, { recursive: true });

// ─── OG 图片 1200×630 ────────────────────────────────────────────────────────
const ogSvg = /* xml */ `
<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <radialGradient id="glow" cx="35%" cy="50%" r="40%">
      <stop offset="0%" stop-color="#6366f1" stop-opacity="0.3"/>
      <stop offset="100%" stop-color="#6366f1" stop-opacity="0"/>
    </radialGradient>
    <pattern id="grid" width="60" height="60" patternUnits="userSpaceOnUse">
      <path d="M 60 0 L 0 0 0 60" fill="none" stroke="#1e293b" stroke-width="1"/>
    </pattern>
  </defs>

  <!-- 背景 -->
  <rect width="1200" height="630" fill="#0f172a"/>
  <rect width="1200" height="630" fill="url(#grid)"/>
  <rect width="1200" height="630" fill="url(#glow)"/>

  <!-- 标题"3X" -->
  <text x="100" y="310"
    font-family="monospace, 'Courier New'"
    font-size="160" font-weight="bold"
    fill="#818cf8">3X</text>

  <!-- 标题"LIVE" -->
  <text x="100" y="460"
    font-family="monospace, 'Courier New'"
    font-size="160" font-weight="bold"
    fill="#f1f5f9">LIVE</text>

  <!-- 强调线 -->
  <rect x="100" y="487" width="280" height="4" rx="2" fill="#6366f1"/>

  <!-- 标语 -->
  <text x="100" y="538"
    font-family="monospace, 'Courier New'"
    font-size="28" fill="#64748b">Multiplayer Bullet Hell Survival</text>

  <!-- URL 徽章 -->
  <rect x="820" y="265" width="280" height="90" rx="12" fill="#1e293b"/>
  <text x="960" y="320" text-anchor="middle"
    font-family="monospace, 'Courier New'"
    font-size="26" font-weight="bold"
    fill="#6366f1">3xlive.app</text>
</svg>
`;

// ─── Apple Touch Icon 180×180 ────────────────────────────────────────────────
const iconSvg = /* xml */ `
<svg xmlns="http://www.w3.org/2000/svg" width="180" height="180" viewBox="0 0 180 180">
  <defs>
    <radialGradient id="glow" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#6366f1" stop-opacity="0.45"/>
      <stop offset="100%" stop-color="#6366f1" stop-opacity="0"/>
    </radialGradient>
  </defs>

  <rect width="180" height="180" fill="#0f172a"/>
  <rect width="180" height="180" fill="url(#glow)"/>

  <text x="90" y="90" text-anchor="middle" dominant-baseline="middle"
    font-family="monospace, 'Courier New'"
    font-size="64" font-weight="bold"
    fill="#818cf8">3X</text>

  <text x="90" y="132" text-anchor="middle"
    font-family="monospace, 'Courier New'"
    font-size="34" font-weight="bold"
    fill="#f1f5f9">LIVE</text>
</svg>
`;

// ─── 渲染输出 ─────────────────────────────────────────────────────────────────
await sharp(Buffer.from(ogSvg)).png({ compressionLevel: 9 }).toFile(resolve(OUT, "og-image.png"));
console.log("✓ public/og-image.png  (1200×630)");

await sharp(Buffer.from(iconSvg))
  .resize(180, 180)
  .png({ compressionLevel: 9 })
  .toFile(resolve(OUT, "apple-touch-icon.png"));
console.log("✓ public/apple-touch-icon.png  (180×180)");
