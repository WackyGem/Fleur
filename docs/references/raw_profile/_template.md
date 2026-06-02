# Raw 数据画像：<dataset>

日期：YYYY-MM-DD

状态：Draft

关联：

- 数据契约：`pipeline/contracts/datasets/<dataset>.yml`
- dbt source：`source('raw', '<dataset>')`
- 生成的 source catalog：`pipeline/elt/models/sources.yml`
- 计划中的 staging model：`pipeline/elt/models/staging/<source>/stg_<source>__<entity>.sql`

## 1. 范围

- source 名称：
- raw 表：
- profiling 命令：
- 行数：
- 数据范围：
- 分区范围：

## 2. 粒度与键

- 观察到的粒度：
- 候选自然键：
- 重复检查：
- 粒度注意事项：

## 3. 字段画像

| 字段 | 类型 | NULL 数 | 空值/占位值 | 去重/样例 | 备注 |
|------|------|---------|-------------|-----------|------|

## 4. 关键字段发现

### 证券代码字段

- 观察到的格式：
- 无效样例：
- 建议 staging 处理：

### 日期与时间字段

- 范围：
- 无效值或占位值：
- 建议 staging 处理：

### 枚举字段

- 取值：
- 未知或异常取值：
- 建议 staging 处理：

### 数值字段

- 最小/最大值：
- 负数/零值/极端值：
- 单位假设：
- 建议 staging 处理：

## 5. 数据质量问题

| 问题 | 严重程度 | 证据 | staging 处理 | 延后处理 |
|------|----------|------|--------------|----------|

## 6. 建议的 Staging 转换

- 重命名：
- 类型转换：
- 标准化：
- NULL 处理：
- 测试：
- YAML 元数据：

## 7. 延后到 Intermediate/Mart

- 跨源 join：
- 需要优先级判断的去重：
- 主数据修正：
- 粒度变化：
- 业务指标逻辑：

## 8. 待确认问题

- [ ] 问题：

## 9. 验收清单

- [ ] 已抽样 raw source。
- [ ] 已记录行数和日期/分区范围。
- [ ] 已评估粒度和候选键。
- [ ] 已完成关键字段画像。
- [ ] 已列出 staging 转换建议。
- [ ] 已列出延后处理事项。
- [ ] 已提出测试或明确豁免。
