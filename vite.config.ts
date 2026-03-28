import { defineConfig } from "vite-plus";
import { cloudflare } from "@cloudflare/vite-plugin";

export default defineConfig({
  fmt: {
    // public/ 包含手写的 JSON 游戏配置文件，不对其格式化
    ignorePatterns: ["public/**", "neko-*/**"],
  },
  plugins: [cloudflare()],
  resolve: {
    alias: {
      "@shared": new URL("src/server/shared/", import.meta.url).pathname,
      "@server": new URL("src/server/", import.meta.url).pathname,
    },
  },
  server: {
    host: true,
    watch: {
      ignored: ["**/target/**"],
    },
  },
  staged: {
    "*": "vp check --fix",
  },
  lint: {
    // public/ 包含 JSON 游戏配置，不是 TypeScript 源码
    ignorePatterns: ["public/**", "src/wasm-client/**", "src/wasm-server/**"],
    options: { typeAware: true, typeCheck: true },
    rules: {
      // 游戏中 console 是合理的：调试叠加层、错误上报
      "no-console": "off",
      // PRNG 刻意使用位运算
      "no-bitwise": "off",
    },
  },
  build: {
    // WASM 文件体积大是正常的，不需要警告
    chunkSizeWarningLimit: 20_000,
    rolldownOptions: {
      output: {
        manualChunks(id: string) {
          if (id.includes("@clerk/")) return "clerk";
          if (id.includes("@fingerprintjs/")) return "fingerprint";
          if (id.includes("/node_modules/")) return "vendor";
        },
      },
    },
  },
});
