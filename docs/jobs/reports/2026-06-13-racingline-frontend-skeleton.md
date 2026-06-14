# Racingline Frontend Skeleton

日期：2026-06-13

范围：`app/racingline/` 前端工程骨架、依赖、环境入口和质量门禁。

## 结论

Racingline 第一版前端工程已创建在 `app/racingline/`，按 shadcn/ui 官方 Vite + `base-nova` 脚手架基线维护，独立 package 管理，不在 `app/` 顶层引入 workspace。

环境变量入口已收敛到仓库根目录 `.env` 和 `.env.example`；`app/racingline/` 下不存在 `.env*` 文件。Vite 通过 `envDir: "../.."` 读取根目录 env，前端代码只读取 `import.meta.env.VITE_REARVIEW_API_BASE_URL`。

## 工程信息

```text
packageManager = npm@11.13.0
framework = Vite
React = 19.2.6
TypeScript = ~6
shadcn style = base-nova
shadcn base = base
iconLibrary = hugeicons
tailwindVersion = v4
```

shadcn project info：

```bash
cd app/racingline
npx shadcn@latest info --json
```

结果摘要：

```text
framework = Vite
style = base-nova
base = base
iconLibrary = hugeicons
components = alert, badge, button, card, checkbox, dialog, empty, field,
             input-group, input, label, select, separator, sheet, skeleton,
             spinner, table, tabs, textarea, tooltip
```

## 环境入口

根目录 `.env.example` 包含：

```text
VITE_REARVIEW_API_BASE_URL=http://127.0.0.1:34057
```

检查命令：

```bash
find app/racingline -maxdepth 5 -name '.env*' -print
```

结果：无输出。

## 前端质量门禁

执行目录：

```bash
cd app/racingline
```

命令和结果：

```text
npm run lint       -> passed
npm run typecheck  -> passed
npm run test       -> 3 files passed, 10 tests passed
npm run build      -> passed
```

`npm run build` 有 Vite chunk size warning：

```text
Some chunks are larger than 500 kB after minification.
```

该 warning 不阻塞第一版验收；后续如引入更多页面或图表能力，再评估路由级 code splitting。

## 后端相关 Rust 门禁

本次实施补齐了 Rearview API，因此追加 Rust workspace 检查：

```bash
cd engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

结果：全部通过。

## 组件边界

- 业务组件位于 `src/components/racingline/`、`src/features/*` 和 `src/routes/`。
- shadcn/ui 官方 CLI 生成的默认组件位于 `src/components/ui/`。
- 长期规则见 `docs/ADR/0011-racingline-frontend-technology-stack.md`：不得手工改写默认 UI 组件；业务 UI 通过独立业务目录组合引用默认组件。

