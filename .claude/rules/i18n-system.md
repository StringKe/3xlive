# 多语言系统规则

## 支持语言

en / zh-CN / ja / ko

## Locale 检测优先级

1. `localStorage("locale")`（用户手动选择）
2. `navigator.language` 浏览器语言
3. 默认 `en`

## 翻译 Key 规范

- 点分路径格式：`ui.xxx.yyy`
- 占位符用数字索引：`{0}`, `{1}`
- 格式化在 Rust 调用方处理，JS 层只返回原始模板字符串
- fetch 失败时返回 key 自身作为 fallback，不报错不崩溃

## 字体加载

每个 locale 对应一个字体文件：

| locale | 字体文件                 |
| ------ | ------------------------ |
| en     | `assets/fonts/en.ttf`    |
| zh-CN  | `assets/fonts/zh-CN.otf` |
| ja     | `assets/fonts/ja.otf`    |
| ko     | `assets/fonts/ko.otf`    |

- 通过 `window.__i18n.fontPath()` 获取路径
- Rust 侧运行时动态加载，禁止硬编码字体路径

## JS 桥接

### `window.__i18n` 接口

| 方法/属性           | 用途                                 |
| ------------------- | ------------------------------------ |
| `locale()`          | 当前 locale 字符串                   |
| `fontPath()`        | 当前 locale 对应字体路径             |
| `t(key)`            | 翻译查询                             |
| `setLocale(locale)` | 切换语言（写 localStorage + reload） |

## 语言切换机制

切换语言 = `localStorage.setItem("locale", newLocale)` + `location.reload()`。无热更新，整页 reload 重新初始化。

## 翻译文件位置

`public/locales/{locale}.json`，Vite 静态服务。

## 新增翻译 Key 流程

1. 在 `public/locales/en.json` 添加 key
2. 同步添加到 zh-CN.json、ja.json、ko.json
3. 在 Rust 侧通过 `js_bridge::t("ui.xxx.yyy")` 调用
4. 四种语言文件 key 必须完全一致

## 禁止事项

- Rust 侧禁止硬编码翻译文本（通过 key 查询）
- Rust 侧禁止包含语言检测逻辑
- 禁止使用 JSON 以外的翻译格式
- 禁止在翻译文件中使用嵌套结构（扁平 key-value）
