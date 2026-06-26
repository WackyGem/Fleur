# Releases

本目录记录 mono-fleur 集成发布快照。单个组件的权威版本仍保留在各自工程文件中，集成 release note 用于回答一次部署包含哪些组件、迁移 head、contract 变更和验证结果。

## 当前发布记录

| Release | Commit | 用途 |
|---|---|---|
| [mono-fleur-2026.06.1](mono-fleur-2026.06.1.md) | `3c20eb538e8aabc1622bbcaada450868b1f6a61c` | 首个版本治理快照，建立 manifest、release note、版本校验和运行时版本暴露 |

## Manifest Schema

`deploy/release-manifest.yml` 必填字段：

| 字段 | 说明 |
|---|---|
| `release` | 集成 release 名称，格式为 `mono-fleur-YYYY.MM.N` |
| `commit` | release manifest 采集时的 Git commit |
| `components` | 可部署或可执行组件版本，不包含 `pipeline` root meta package |
| `database_heads` | Alembic target 实际 revision head，至少包含 `pipeline` 和 `rearview` |
| `target_schema_heads` | 每个 target 最后一个会执行 DDL 的 migration；用于区分 no-op target migrations |
| `contracts.registry_commit` | contract registry 对应 commit |
| `contracts.changed_datasets` | 本次 contract 变更 dataset 列表，没有变更时为空列表 |
| `verification` | 发布前验证命令的结果状态 |

## Version Impact 模板

```text
Version impact:
- Components:
  - <component>: <old> -> <new>, <major|minor|patch|none>, <reason>
- Dataset contracts:
  - <dataset>: <old> -> <new>, <reason>
- Alembic heads:
  - <target>: <old> -> <new>
- Release manifest: <updated|not updated>, <reason>
- Runtime version exposure: <updated|not updated>, <reason>
- Tags: <none|component tag|integration tag>, <reason>
```

## Tag 前检查

创建集成 tag 前必须确认：

1. `git status --short` 工作区干净，或用户明确指定已提交 commit。
2. `deploy/release-manifest.yml` 已更新并通过 `make versions-check`。
3. 对应 release note 已记录组件版本、Alembic head、target schema head、contract 变化和验证结果。
4. `git tag --list | sort` 中不存在同名 tag。
