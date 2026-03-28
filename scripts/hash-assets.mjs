#!/usr/bin/env node
/**
 * 构建后脚本：对所有静态资源（index.html 和 _version.json 除外）
 * 添加内容 hash，用于 CDN 缓存破坏。
 * 生成 /asset-manifest.json 供运行时解析 URL。
 */
import { createHash } from "node:crypto";
import {
  existsSync,
  readFileSync,
  readdirSync,
  renameSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, extname, join, relative } from "node:path";

const DIST = "dist/client";
const INDEX = join(DIST, "index.html");
const MANIFEST = join(DIST, "asset-manifest.json");

if (!existsSync(INDEX)) {
  console.error(`ERROR: ${INDEX} not found. Run build first.`);
  process.exit(1);
}

/** @type {Record<string, string>} 旧路径 -> 新路径（均带前导 /） */
const mappings = {};

function hashFile(filePath) {
  const content = readFileSync(filePath);
  return createHash("sha256").update(content).digest("hex").slice(0, 8);
}

function hashAndRename(filePath) {
  if (!existsSync(filePath)) return;

  const hash = hashFile(filePath);
  const dir = dirname(filePath);
  const ext = extname(filePath);
  const name = basename(filePath, ext);
  const newName = `${name}.${hash}${ext}`;
  const newPath = join(dir, newName);

  renameSync(filePath, newPath);

  const oldRel = `/${relative(DIST, filePath)}`;
  const newRel = `/${relative(DIST, newPath)}`;
  mappings[oldRel] = newRel;
  console.log(`  ${oldRel} -> ${newRel}`);
}

function collectFiles(dir) {
  const results = [];
  if (!existsSync(dir)) return results;
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      results.push(...collectFiles(full));
    } else {
      results.push(full);
    }
  }
  return results;
}

function applyMappings(filePath) {
  if (!existsSync(filePath)) return;
  let content = readFileSync(filePath, "utf-8");
  for (const [oldPath, newPath] of Object.entries(mappings)) {
    content = content.replaceAll(oldPath, newPath);
  }
  writeFileSync(filePath, content);
}

// 1. 字体文件
for (const f of readdirSync(join(DIST, "assets", "fonts")).filter((f) =>
  /\.(woff2|otf|ttf)$/.test(f),
)) {
  hashAndRename(join(DIST, "assets", "fonts", f));
}

// 2. 多语言文件
for (const f of readdirSync(join(DIST, "locales")).filter((f) => f.endsWith(".json"))) {
  hashAndRename(join(DIST, "locales", f));
}

// 3. 配置文件（递归）
for (const f of collectFiles(join(DIST, "configs"))) {
  hashAndRename(f);
}

// 4. 图标/图片文件
for (const f of [
  "favicon.ico",
  "favicon.svg",
  "favicon-16x16.png",
  "favicon-32x32.png",
  "apple-touch-icon.png",
  "android-chrome-192x192.png",
  "android-chrome-512x512.png",
  "og-image.png",
  "icons.svg",
]) {
  hashAndRename(join(DIST, f));
}

// 5. 其他 HTML 页面
for (const f of ["privacy.html", "terms.html"]) {
  hashAndRename(join(DIST, f));
}

// 6. site.webmanifest — 先更新图标引用，再 hash
const webmanifest = join(DIST, "site.webmanifest");
if (existsSync(webmanifest)) {
  applyMappings(webmanifest);
  hashAndRename(webmanifest);
}

// 注意：_version.json 不 hash，VersionChecker 以固定 URL 轮询它
// 注意：asset-manifest.json 不 hash，启动时带 cache-bust param 加载
// 注意：assets/（Vite 输出）已有内容 hash

console.log("\n=== Updating index.html references ===");
applyMappings(INDEX);

console.log("=== Generating asset-manifest.json ===");
const sorted = Object.fromEntries(Object.entries(mappings).sort(([a], [b]) => a.localeCompare(b)));
writeFileSync(MANIFEST, JSON.stringify(sorted, null, 2) + "\n");
console.log(`Written ${MANIFEST} with ${Object.keys(mappings).length} entries`);
console.log("=== Done ===");
