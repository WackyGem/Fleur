# Dagster Slack 资产失败通知实施计划

日期：2026-05-31

关联设计文档：

- `docs/RFC/0008-dagster-slack-asset-failure-alerting.md`
- `docs/architecture/scheduler-architecture.md`
- `docs/architecture/scheduler-module-boundaries.md`

## 1. 目标

本计划用于按 RFC 0008 在 `pipeline/scheduler` 接入 Slack 资产失败通知。

实施目标：

- 在任意当前 Dagster asset job run failure 后发送 Slack 告警。
- 使用 `dagster-slack` 官方集成库，并以项目级 `SlackAlertResource` 补足 `SLACK_HTTP_PROXY` 专用代理配置。
- 新增跨数据源 `slack_asset_failure_sensor`，不修改任何 source bundle 的 assets、jobs、schedules。
- Slack token、channel、proxy、Dagster Webserver URL 都通过 `defs/config/env.py` 和 resource 边界注入。
- 告警失败只记录 sensor 日志，不改变原始失败 run 的状态，不触发额外资产执行。

非目标：

- 不接入 Dagster Plus alert policy。
- 不使用每个 asset/job 绑定 hook 的方式。
- 不把 `SLACK_HTTP_PROXY` 扩散为进程级 `HTTP_PROXY` 或 `HTTPS_PROXY`。
- 不在本轮实现成功、重试、SLA miss、数据为空等非 run failure 告警。

## 2. 当前代码基线

当前 scheduler 装配入口：

- `pipeline/scheduler/src/scheduler/definitions.py` 只 re-export `scheduler.defs.definitions.defs`。
- `pipeline/scheduler/src/scheduler/defs/definitions.py` 通过 `SOURCE_BUNDLES` 聚合 `sina`、`jiuyan`、`ths`、`baostock`、`eastmoney`。
- `SOURCE_BUNDLES` 只提供 assets、jobs、schedules；当前没有 sensor 聚合契约。

当前可直接复用的落点：

| 目录或文件 | 当前职责 | 本轮使用方式 |
|------------|----------|--------------|
| `defs/automation/` | 跨数据源 Dagster automation 工厂 | 新增 Slack failure sensor 和消息构造 |
| `defs/config/env.py` | 集中声明 `dg.EnvVar` 常量 | 新增 Slack 和 Webserver URL EnvVar |
| `defs/resources/` | Dagster resource 适配层 | 新增 `SlackAlertResource` |
| `defs/definitions.py` | repository 级 definitions 装配 | 注册 `slack` resource 和 sensor |
| `tests/integration/test_definitions_and_schedules.py` | 验证 definitions 与 source bundles 契约 | 扩展 automation/resource 注册断言 |
| `tests/integration/test_architecture_boundaries.py` | 架构边界文本扫描 | 增加 Slack env 读取边界断言 |

当前已注册 resources：

- `s3_io_manager`
- `s3_settings`
- `image_object_store`
- `industry_image_repository`
- `jiuyan_ocr_settings`
- `baostock_client_factory`
- `http_client_factory`

接入后新增：

- resource：`slack`
- sensor：`slack_asset_failure_sensor`

当前 `.env.example` 已包含：

- `SLACK_BOT_TOKEN`
- `SLACK_CHANNEL_ID`
- `SLACK_HTTP_PROXY`

本轮需要补充：

- `DAGSTER_WEBSERVER_BASE_URL`
- `DAGSTER_CODE_LOCATION_NAME`

## 3. 设计约束

### 3.1 模块边界

新增代码必须遵守现有边界：

- Slack 环境变量只允许在 `defs/config/env.py` 中声明，并由 `defs/resources/slack.py` 消费。
- `defs/automation/slack_alerts.py` 不读取环境变量，不导入任何具体 source 模块。
- `defs/definitions.py` 只做装配，不包含消息格式和 Slack SDK 调用细节。
- `SourceBundle` 契约不变，不为一个全局告警 sensor 扩展 bundle 字段。
- source 业务代码不 import `dagster_slack`、`slack_sdk` 或 Slack env。

### 3.2 Dagster 集成方式

按 Dagster 当前文档，sensor 可以通过 `Definitions(resources=...)` 获取 resource，并可用函数参数声明 resource 依赖。RFC 0008 的目标形态为：

```python
@dg.run_failure_sensor(
    name="slack_asset_failure_sensor",
    default_status=dg.DefaultSensorStatus.RUNNING,
    minimum_interval_seconds=30,
)
def slack_asset_failure_sensor(
    context: dg.RunFailureSensorContext,
    slack: SlackAlertResource,
) -> None:
    ...
```

`slack_asset_failure_sensor` 不返回 `RunRequest`，只发送通知副作用。

### 3.3 Slack Resource

`SlackAlertResource` 必须继承 `dagster_slack.SlackResource`，而不是完全自建 Slack SDK resource。

推荐字段：

- `token`: 默认来自 `SLACK_BOT_TOKEN`
- `channel_id`: 默认来自 `SLACK_CHANNEL_ID`
- `http_proxy`: 默认来自 `SLACK_HTTP_PROXY`，空字符串归一化为 `None`
- `webserver_base_url`: 默认来自 `DAGSTER_WEBSERVER_BASE_URL`，空字符串归一化为 `None`
- `code_location_name`: 默认来自 `DAGSTER_CODE_LOCATION_NAME`，用于 Slack 消息中的环境或 code location 字段

`get_client()` 显式把 proxy 传给 Slack SDK `WebClient`，避免影响 S3、HTTP 数据源、BaoStock TCP 等其他外部连接。

## 4. 阶段 A：依赖与配置

目标：先让项目具备 Dagster Slack 集成依赖和配置入口。

### A.1 添加依赖

修改位置：

- `pipeline/scheduler/pyproject.toml`
- `pipeline/uv.lock`

操作：

1. 在 scheduler dependencies 中添加 `dagster-slack==0.29.6`。
2. 在 `pipeline/` 下执行依赖同步，更新 `uv.lock`。

推荐命令：

```bash
cd pipeline
uv sync --all-packages --all-groups
```

完成标准：

- `uv run python -c "import dagster_slack"` 成功。
- `pipeline/uv.lock` 中出现 `dagster-slack` 和其 Slack SDK 依赖。

### A.2 扩展环境变量声明

修改位置：

- `pipeline/scheduler/src/scheduler/defs/config/env.py`
- `.env.example`

操作：

1. 在 `env.py` 中新增：
   - `SLACK_BOT_TOKEN = dg.EnvVar("SLACK_BOT_TOKEN")`
   - `SLACK_CHANNEL_ID = dg.EnvVar("SLACK_CHANNEL_ID")`
   - `SLACK_HTTP_PROXY = dg.EnvVar("SLACK_HTTP_PROXY")`
   - `DAGSTER_WEBSERVER_BASE_URL = dg.EnvVar("DAGSTER_WEBSERVER_BASE_URL")`
   - `DAGSTER_CODE_LOCATION_NAME = dg.EnvVar("DAGSTER_CODE_LOCATION_NAME")`
2. 在 `.env.example` 的 `# Dagster` 或 `# SLACK` 段补充：
   - `DAGSTER_WEBSERVER_BASE_URL=http://localhost:3000`
   - `DAGSTER_CODE_LOCATION_NAME=scheduler`
3. 保留 `SLACK_HTTP_PROXY=` 支持无代理环境。

完成标准：

- 业务模块不直接调用 `dg.EnvVar("SLACK_...")`、`dg.EnvVar("DAGSTER_WEBSERVER_BASE_URL")` 或 `dg.EnvVar("DAGSTER_CODE_LOCATION_NAME")`。
- `.env.example` 不包含真实 token。

## 5. 阶段 B：Slack Resource

目标：在 resource 边界封装 Slack 官方 resource、channel、proxy 和 run 链接配置。

新增文件：

- `pipeline/scheduler/src/scheduler/defs/resources/slack.py`

推荐实现形态：

```python
from __future__ import annotations

from dagster_slack import SlackResource
from slack_sdk.web.client import WebClient

from scheduler.defs.config import env


class SlackAlertResource(SlackResource):
    token: str = env.SLACK_BOT_TOKEN
    channel_id: str = env.SLACK_CHANNEL_ID
    http_proxy: str = env.SLACK_HTTP_PROXY
    webserver_base_url: str = env.DAGSTER_WEBSERVER_BASE_URL
    code_location_name: str = env.DAGSTER_CODE_LOCATION_NAME

    def get_client(self) -> WebClient:
        return WebClient(token=self.token, proxy=_blank_to_none(self.http_proxy))
```

实际实现注意：

- `token` 必须在 `SlackAlertResource` 中显式设置默认值 `env.SLACK_BOT_TOKEN`，保证 `defs/definitions.py` 可以直接注册 `SlackAlertResource()`。
- 如果 `SlackResource.token` 不能在子类中直接覆盖默认值，应按 Pydantic/Dagster resource 约束选择兼容实现，但仍必须让 token 从 `defs/config/env.py` 的 `SLACK_BOT_TOKEN` 获取。
- `_blank_to_none()` 作为私有纯函数，统一处理 `None` 和空白字符串。
- `webserver_base_url` 只负责提供配置，不在 resource 内拼接 run URL。
- `code_location_name` 只负责提供消息中的环境或 code location 标识；不要在 sensor 中直接读取环境变量。

新增测试：

- `pipeline/scheduler/tests/unit/resources/test_slack.py`

测试项：

- `SlackAlertResource` 是 `dagster_slack.SlackResource` 的子类。
- 默认字段 `token`、`channel_id`、`http_proxy`、`webserver_base_url`、`code_location_name` 来自 `scheduler.defs.config.env`。
- `get_client()` 创建 `WebClient` 时传入 token 和 proxy。
- `http_proxy=""` 或空白字符串时传入 `proxy=None`。
- `webserver_base_url=""` 时后续消息构造可识别为无链接。

完成标准：

- 测试不真实访问 Slack。
- Slack SDK 只在 resource 边界被直接使用。

## 6. 阶段 C：Slack Failure Sensor 与消息构造

目标：新增跨数据源 run failure sensor，并把复杂逻辑拆为可测纯函数。

新增文件：

- `pipeline/scheduler/src/scheduler/defs/automation/slack_alerts.py`

推荐拆分：

| 函数或对象 | 职责 |
|------------|------|
| `SlackFailureMessage` | 保存 fallback text 和 Block Kit blocks |
| `build_slack_failure_message(...)` | 从 context-like 数据构造 Slack 消息 |
| `build_run_url(...)` | 根据 `webserver_base_url` 和 run id 拼接 Dagster run 链接 |
| `truncate_error(...)` | 错误摘要截断，默认 1500 字符 |
| `slack_asset_failure_sensor(...)` | Dagster sensor 外壳，调用 resource 发送消息 |

消息字段：

- job name
- environment 或 code location
- run id
- partition key
- failed steps
- asset selection 或资产候选
- error 摘要
- Dagster run URL

提取顺序：

1. 使用 `context.get_step_failure_events()` 获取失败 step 和错误摘要候选。
2. 使用 `context.dagster_run.asset_selection` 获取本次 run 资产候选范围。
3. 使用 `context.partition_key` 获取分区。
4. 使用 `context.failure_event.message` 作为 run 级错误兜底。

实现约束：

- 多 asset run 不要声称唯一失败 asset；使用“资产候选”和 failed steps 表达。
- environment 或 code location 字段来自 `SlackAlertResource.code_location_name`，不得在 sensor 或消息构造函数中直接读取环境变量。
- Slack 发送异常需要捕获并通过 `context.log.exception(...)` 或等价日志记录。
- sensor 函数本身不要把 Slack 发送异常继续抛出。
- `minimum_interval_seconds` 使用 30。
- `default_status` 使用 `dg.DefaultSensorStatus.RUNNING`。

新增测试：

- `pipeline/scheduler/tests/unit/automation/test_slack_alerts.py`

测试项：

- fallback text 包含 job name 和 run id。
- blocks 包含 code location、job、run、partition、failed steps、assets、error。
- partition 缺失时展示 `-`。
- asset selection 缺失时展示 `-`。
- 长错误消息会截断。
- webserver URL 缺失时不生成无效链接。
- Slack 发送异常被吞掉并记录日志。

完成标准：

- 消息构造大部分测试不依赖真实 Dagster context。
- 至少一个测试覆盖 sensor 外壳调用 fake Slack resource。

## 7. 阶段 D：Definitions 装配

目标：把 Slack resource 和 sensor 接入 repository 级 definitions，保持 source bundle 契约不变。

修改位置：

- `pipeline/scheduler/src/scheduler/defs/definitions.py`

操作：

1. import `slack_asset_failure_sensor`。
2. import `SlackAlertResource`。
3. 在 `dg.Definitions(...)` 中新增：

```python
sensors=[slack_asset_failure_sensor],
resources={
    ...
    "slack": SlackAlertResource(),
}
```

注意：

- 不修改 `SOURCE_BUNDLES`。
- 不把 sensor 放进任一 source `definitions.py`。
- 不改变现有 assets、jobs、schedules 聚合顺序。

完成标准：

- `scheduler_defs.load_fn()` 能正常加载。
- `loaded_defs.resources` 包含 `slack`。
- Python 层 definitions 测试只断言显式注册的 `slack_asset_failure_sensor`。
- Dagster 自动生成的 `default_automation_condition_sensor` 通过 `uv run dg list defs --json` 验证，不使用 `loaded_defs.sensors` 断言。

## 8. 阶段 E：测试与架构护栏

目标：用单元测试、definitions 集成测试和文本边界测试保护新增能力。

### E.1 扩展 definitions 集成测试

修改位置：

- `pipeline/scheduler/tests/integration/test_definitions_and_schedules.py`

操作：

1. 将 `test_registered_definitions_match_source_bundles` 扩展为同时断言 automation。
2. 保持 source bundle assets/jobs/schedules 断言不变。
3. 新增 resource 断言：
   - `slack`
4. 新增 sensor 断言：
   - `slack_asset_failure_sensor`
5. 不在 Python 层集成测试中断言 `default_automation_condition_sensor`，该 sensor 由 Dagster 根据 automation condition 生成，使用 `dg list defs --json` 作为验收验证。

完成标准：

- 新增 sensor 不改变任何 source bundle contract。

### E.2 扩展架构边界测试

修改位置：

- `pipeline/scheduler/tests/integration/test_architecture_boundaries.py`

新增断言：

- 除 `defs/config/env.py` 外，不允许出现 `dg.EnvVar("SLACK_`。
- 除 `defs/config/env.py` 外，不允许出现 `dg.EnvVar("DAGSTER_WEBSERVER_BASE_URL")` 或 `dg.EnvVar("DAGSTER_CODE_LOCATION_NAME")`。
- 除 `defs/resources/slack.py` 外，不允许直接 import `dagster_slack` 或 `slack_sdk`。
- `defs/sources/` 和 `defs/baostock/` 中不允许出现 `SLACK_`。

完成标准：

- Slack 配置和 SDK 调用被限制在 config/resource 边界。

### E.3 单元测试目录

新增目录或文件：

- `pipeline/scheduler/tests/unit/automation/test_slack_alerts.py`
- `pipeline/scheduler/tests/unit/resources/test_slack.py`

如果 `tests/unit/resources/` 不存在，新增该目录并保持与当前测试命名风格一致。

## 9. 阶段 F：本地验证与受控告警演练

目标：确认 definitions 可加载、sensor 可见、Slack 消息能在受控失败场景发送。

### F.1 静态与单元验证

在 `pipeline/` 下执行：

```bash
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
```

完成标准：

- 所有检查通过。
- pytest 不依赖真实 Slack token 或网络。

### F.2 Dagster definitions 验证

在 `pipeline/scheduler/` 下执行：

```bash
uv run dg check defs
uv run dg list defs --json
```

重点检查：

- resources 包含 `slack`。
- sensors 包含 `slack_asset_failure_sensor`。
- `default_automation_condition_sensor` 仍存在。
- assets、jobs、schedules 数量不因 Slack 接入改变。

### F.3 受控失败演练

操作：

1. 配置真实但非生产敏感的 Slack app token、channel ID 和 proxy。
2. 配置 `DAGSTER_CODE_LOCATION_NAME` 和可选 `DAGSTER_WEBSERVER_BASE_URL`，两者都必须通过 `.env` 和 `defs/config/env.py` 进入 resource。
3. 启动 Dagster：

```bash
cd pipeline/scheduler
uv run dg dev
```

4. 使用受控失败 run 验证 Slack 消息。
5. 验证消息包含 code location、job、run、partition、failed steps、资产候选、错误摘要和 run URL。
6. 演练完成后删除任何临时失败定义，不把测试 job 留在生产 definitions。

完成标准：

- Slack channel 收到预期告警。
- sensor 日志中没有未捕获异常。

## 10. 验收标准

功能验收：

- 任意当前 repository 内 job run failure 可触发 `slack_asset_failure_sensor`。
- Slack 消息包含 code location、job、run、partition、failed steps、资产候选、错误摘要和 run URL 等最小排障字段。
- Slack API 调用使用 `SLACK_HTTP_PROXY` 专用代理。
- 未配置 `DAGSTER_WEBSERVER_BASE_URL` 时消息仍可发送，只是不包含可点击 run 链接。

架构验收：

- `SOURCE_BUNDLES` 顺序和每个 bundle 的 assets/jobs/schedules 不变。
- `SourceBundle` 契约不变。
- `defs/automation/slack_alerts.py` 不依赖具体 source。
- Slack env 读取只集中在 `defs/config/env.py` 和 resource 默认值路径。
- `DAGSTER_WEBSERVER_BASE_URL` 和 `DAGSTER_CODE_LOCATION_NAME` 读取只集中在 `defs/config/env.py` 和 resource 默认值路径。
- `slack_asset_failure_sensor` 与 `default_automation_condition_sensor` 并存。

测试验收：

- Slack resource 单测覆盖继承关系、proxy 归一化和 client 构造。
- Slack alert 单测覆盖 code location 展示、消息构造、截断、字段缺失和发送异常处理。
- definitions 集成测试覆盖 `slack` resource 和 `slack_asset_failure_sensor`。
- 架构边界测试覆盖 Slack SDK/env 访问边界。

## 11. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Slack token、channel 或 scope 配置错误 | run failure 无法通知到 channel | sensor 捕获异常并记录 Slack API 错误；部署检查 bot 是否加入 channel |
| proxy 地址只在宿主机可用，容器内不可达 | Slack API 超时 | 在 Dagster 进程所在运行环境验证 `SLACK_HTTP_PROXY` 可达 |
| 多 asset run 被误读为单一资产失败 | 排障方向偏差 | 第一阶段只展示资产候选和 failed steps，不做唯一资产断言 |
| sensor 发送异常导致 sensor evaluation failure | 产生第二个运维事件 | Slack 发送异常只记录日志，不重新抛出 |
| `.env` 缺少 Slack 或 Dagster code location 变量导致本地 definitions 加载失败 | 开发者无法启动 Dagster | `.env.example` 保持占位；如后续需要可新增 `SLACK_ALERT_ENABLED` 开关 |
| `dagster-slack` 与 Dagster core 版本不匹配 | import 或 runtime 失败 | 使用与 `dagster==1.13.6` 对应的 `dagster-slack==0.29.6` |

## 12. 回滚方案

如上线后需要快速回滚：

1. 从 `defs/definitions.py` 移除 `slack_asset_failure_sensor` 和 `slack` resource 注册。
2. 保留 `dagster-slack` 依赖和新增测试代码可不影响运行；如需彻底回滚，再移除依赖、resource、sensor 和测试。
3. 不需要修改 source bundle，因为本方案不触碰业务资产、jobs、schedules。

快速禁用的替代方式：

- 将 sensor default status 改为 `STOPPED`，或在 Dagster UI 中停用 `slack_asset_failure_sensor`。

## 13. 开放问题

1. 是否需要 `SLACK_ALERT_ENABLED`，允许 CI 或本地环境加载 definitions 但默认不发送 Slack？
2. 是否需要按 job tag、asset tag 或 source 名称过滤低优先级告警？
3. 是否需要对同一 run 的重复 sensor evaluation 做持久化幂等？
4. 是否需要把部分失败阈值、空数据、schema drift 等业务异常扩展为 warning 告警？
5. 是否需要为生产 Dagster Webserver URL 制定统一环境变量命名和部署注入方式？
