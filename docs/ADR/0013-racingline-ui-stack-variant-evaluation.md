# ADR 0013: Racingline UI 栈变体评估

状态：Proposed

日期：2026-06-17

## 背景

`racingline` 的正式前端技术栈已经由 ADR 0011 固化：

- [app/racingline/package.json](../../app/racingline/package.json)
- [app/racingline/components.json](../../app/racingline/components.json)
- [ADR 0011: Racingline 前端技术栈和工程边界](0011-racingline-frontend-technology-stack.md)

当前 `racingline_new` 是并行原型工程，其脚手架来自 shadcn 的 `b3lEPaZ6H` preset，实际生成结果与 `racingline` 正式栈不同：

- [app/racingline_new/package.json](../../app/racingline_new/package.json)
- [app/racingline_new/components.json](../../app/racingline_new/components.json)

主要差异包括：

| 维度 | `racingline` 正式栈 | `racingline_new` 当前栈 |
|---|---|---|
| shadcn style | `base-nova` | `base-lyra` |
| primitive base | `base` | `radix` |
| icon library | `hugeicons` | `lucide` |
| 字体 | `Geist` | `IBM Plex Sans` |
| 主题 | `neutral` | `taupe` |
| menuColor | `default` | `inverted` |
| UI primitive 依赖 | `@base-ui/react` | `@base-ui/react` |

这个差异不是单纯的主题切换，而是会影响组件 API、图标导入、基础 primitive 依赖和业务组件迁移路径。

## 问题

是否应把 `racingline_new` 当前的 UI 栈变体推广为 Racingline 的正式前端技术栈？

## 评估

### 优点

1. `base-lyra` 的视觉风格更适合做高密度工作台的快速原型。
2. `IBM Plex Sans` + `taupe` 的组合在当前页面气质上更偏策略实验室，而不是通用管理后台。
3. `lucide` 在常规语义图标上覆盖广，原型阶段较容易快速拼页面。

### 代价

1. 这会把 `racingline` 与 `racingline_new` 的样式和部分组件实现路径拉开，若推广到正式工程，仍需迁移或重适配。
2. 现有 `racingline` 页面和业务组件已经基于 ADR 0011 的栈建立，切换后需要重写或重适配。
3. 图标库从 `Hugeicons` 切到 `Lucide`，会造成大量图标名、语义和导入路径变化。
4. 这次变体只是一个新工程脚手架结果，不是经过完整用户验证、截图验收和迁移计划的长期决策。

## 决策

当前不接受把 `racingline_new` 的 UI 栈变体直接升级为 `racingline` 的正式技术栈。

正式工程继续遵循 ADR 0011：

- `base-nova`
- `@base-ui/react`
- `Hugeicons`
- `Geist`
- `neutral` 主题与默认 menu 配置

`racingline_new` 可以继续作为原型和交互验证工程使用当前栈，但它的结果只能作为 UI 观察材料，不能自动反推正式工程改栈。

## 后果

1. `racingline_new` 当前栈仅用于原型验证。
2. 如果未来希望把该变体推广到正式工程，必须先完成：
   - 浏览器验收
   - 业务组件迁移评估
   - 图标替换评估
   - 基础 primitive 替换评估
   - 新的迁移计划或更新 ADR 0011
3. 现有 `racingline` 页面不需要立即改栈。
4. `racingline_new` 的原型代码不得默认沉淀为生产实现。

## 相关文档

- [ADR 0011: Racingline 前端技术栈和工程边界](0011-racingline-frontend-technology-stack.md)
- [RFC 0023: Racingline 前端原型驱动开发流程](../RFC/0023-racingline-frontend-prototype-led-development.md)
- [System: Racingline](../systems/racingline.md)
