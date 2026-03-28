// 资源路径解析器：将原始路径映射到 hash 后的路径
// asset-manifest.json 由 hash-assets.mjs 在构建时生成

let manifest: Record<string, string> = {};

export async function loadAssetManifest(): Promise<void> {
  // dev 模式下无 manifest，跳过避免控制台 404
  if (import.meta.env.DEV) return;
  try {
    const res = await fetch(`/asset-manifest.json?t=${Date.now()}`);
    if (res.ok) manifest = await res.json();
  } catch {
    // manifest 加载失败，所有路径原样返回
  }
}

/** 将原始资源路径解析为 hash 后的路径，无映射时原样返回 */
export function resolveAsset(path: string): string {
  return manifest[path] ?? path;
}
