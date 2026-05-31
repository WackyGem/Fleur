# RFC 0008: Dagster Slack 资产失败通知接入设计

状态：草案（2026-05-31）

## 摘要

本文档定义 `pipeline/scheduler` 接入 Slack 告警的设计。目标是在 Dagster 资产执行失败时，将失败信息发送到 Slack，并在当前网络环境下通过 HTTP proxy 访问 Slack 服务。

核心决策：

1. **使用 `dagster-slack` 作为 Slack 官方集成库**：该库提供 `SlackResource`、`make_slack_on_run_failure_sensor`、`slack_on_failure` 等能力，底层封装 Slack Python SDK 的 `chat.postMessage`。
2. **基于 `dagster_slack.SlackResource` 做项目级扩展**：保留 Dagster 官方 Slack resource 作为集成入口，只补足本项目需要的 `SLACK_HTTP_PROXY` 显式代理配置。
3. **第一阶段采用自定义 `@run_failure_sensor`**：不直接使用 `make_slack_on_run_failure_sensor`，因为该 helper 内部直接创建 Slack `WebClient`，不接收 Dagster resource，也不暴露 proxy 参数。
4. **Slack 客户端注册为 Dagster resource**：环境变量读取仍集中在 `config/` 和 `resources/` 边界内，顶层 `defs()` 只做资源和 sensor 装配。
5. **通知粒度以 run failure 触发、资产失败为内容**：Dagster OSS 的失败 sensor 以 run 状态变化为触发点，消息中通过 step failure events、asset selection、partition tags 提取失败资产上下文。
6. **代理使用业务专用变量 `SLACK_HTTP_PROXY`**：不要求把代理写入全局 `HTTP_PROXY/http_proxy`，避免影响 S3、BaoStock、HTTP 数据源等其他外部连接。

## 背景

当前 scheduler 架构采用 `SourceBundle` 聚合模式：

- `src/scheduler/defs/definitions.py` 是统一 definitions 装配入口。
- 各数据源在自己的 `definitions.py` 中导出 `SourceBundle`，由 `SOURCE_BUNDLES` 聚合 assets、jobs、schedules。
- 当前注册 resources 包括 S3、图片对象存储、数据库 repository、OCR、BaoStock、HTTP client factory。
- 当前已有 Dagster 自动生成的 `default_automation_condition_sensor`，来源是部分 asset 上的 automation condition；当前没有自定义失败告警 sensor。

环境变量统一配置在根目录 `.env`，业务模块不直接读取环境变量，应通过 `config/`、resource、factory 或 gateway 注入配置。现有 `.env` 已包含 Slack 接入所需变量名：

| 变量 | 用途 |
|------|------|
| `SLACK_BOT_TOKEN` | Slack Bot User OAuth Token，通常为 `xoxb-...` |
| `SLACK_CHANNEL_ID` | 接收告警的 Slack channel ID，优先使用 channel ID 而不是 `#name` |
| `SLACK_HTTP_PROXY` | 访问 Slack API 的 HTTP/HTTPS proxy URL |

## 当前事实基线

以 `cd pipeline/scheduler && uv run dg list defs --json` 核验，当前 Dagster definitions 基线如下：

| 类型 | 当前内容 |
|------|----------|
| Assets | 18 个，全部在 group `s3_sources`，asset key 形如 `source/<source>__<name>` |
| Jobs | 9 个：`baostock__daily_job`、`eastmoney__daily_job`、`jiuyan__action_field_compacted_job`、`jiuyan__action_field_daily_job`、`jiuyan__industry_list_snapshot_job`、`jiuyan__industry_ocr_pipeline_job`、`sina__trade_calendar_job`、`ths__limit_up_pool_compacted_job`、`ths__limit_up_pool_daily_job` |
| Schedules | 7 个 source schedule |
| Resources | 7 个：`s3_io_manager`、`s3_settings`、`image_object_store`、`industry_image_repository`、`jiuyan_ocr_settings`、`baostock_client_factory`、`http_client_factory` |
| Sensors | 1 个：`default_automation_condition_sensor` |

Slack 接入后的预期基线：

| 类型 | 预期变化 |
|------|----------|
| Assets | 不变 |
| Jobs | 不变 |
| Schedules | 不变 |
| Resources | 新增 `slack`，类型为 `scheduler.defs.resources.slack.SlackAlertResource` |
| Sensors | 新增 `slack_asset_failure_sensor`，与 `default_automation_condition_sensor` 并存 |

这意味着 Slack 告警属于跨数据源 automation，不应修改任一 source bundle 的 assets、jobs、schedules。

## Dagster Slack 能力调研

Dagster 官方 `dagster-slack` 集成提供三类能力：

| 能力 | 适用场景 | 本项目选择 |
|------|----------|------------|
| `SlackResource` | 在 asset、op、schedule、sensor 中调用 Slack Web API | 采用 |
| `make_slack_on_run_failure_sensor` | 快速创建 run failure Slack sensor | 暂不直接采用 |
| `slack_on_failure` hook | step 失败时发送 hook 告警 | 不采用 |

`SlackResource` 是本项目应采用的官方集成入口。它的 `get_client()` 返回 Slack Python SDK `WebClient`，适合在 Dagster asset、op、schedule、sensor 中发送消息。当前官方实现只接收 `token`，`get_client()` 等价于创建 `WebClient(token)`；如果没有传入 proxy，Slack Python SDK 会自动读取标准 `HTTPS_PROXY`、`https_proxy`、`HTTP_PROXY`、`http_proxy`。

`make_slack_on_run_failure_sensor` 可以直接根据 run failure 发送 Slack 消息，并支持 `text_fn`、`blocks_fn`、`monitored_jobs`、`monitor_all_code_locations`、`webserver_base_url` 等参数。但它的参数以 `slack_token` 为主，内部直接创建 Slack `WebClient`，不接收已注册的 Dagster `SlackResource`，也不暴露本项目需要的 `SLACK_HTTP_PROXY` 配置入口。虽然可以把 `SLACK_HTTP_PROXY` 复制到进程级 `HTTPS_PROXY` 来让 helper 间接生效，但这会改变整个 Dagster 进程的代理语义，不适合作为默认设计。

因此第一阶段建议采用 `dagster_slack.SlackResource`，并通过一个很薄的项目级子类或包装 resource 显式传入 proxy，再用 `@dg.run_failure_sensor` 实现告警逻辑。这样是“使用 dagster-slack 集成库”，不是自建 Slack 集成；自定义部分只负责项目特有的代理配置、消息格式和资产上下文。

## 设计目标

1. 任意 source bundle 的 asset job 失败后发送 Slack 消息。
2. 消息包含最小但足够定位问题的信息：
   - 环境或 code location；
   - job name；
   - run id；
   - partition key；
   - 失败 step；
   - 失败资产候选；
   - 错误摘要；
   - Dagster Webserver run 链接。
3. Slack token、channel、proxy 全部来自 `.env`。
4. Slack 接入不改变现有 source bundle、asset、schedule 的业务代码。
5. Slack 发送失败不能改变原始 Dagster run 的失败状态，也不能触发额外资产执行。

## 非目标

1. 不在本 RFC 中实现代码。
2. 不引入 Dagster Plus alert policy 作为第一阶段方案。
3. 不对成功、重试、长期运行、SLA miss 发送通知。
4. 不把 Slack 通知做成每个资产的 hook 装饰器。
5. 不把 `SLACK_HTTP_PROXY` 扩散为全局 `HTTP_PROXY`。

## 推荐架构

新增一个跨数据源 automation 模块，保持现有 module boundaries：

```text
pipeline/scheduler/src/scheduler/defs/
├── automation/
│   ├── schedules.py
│   └── slack_alerts.py          # 新增：failure sensor、消息构造
├── config/
│   └── env.py                   # 新增 Slack EnvVar 常量
├── resources/
│   └── slack.py                 # 新增 SlackAlertResource
└── definitions.py               # 注册 resource 和 sensor
```

`automation/slack_alerts.py` 属于跨数据源 Dagster automation，不依赖具体数据源。它只依赖 Dagster sensor context 和 Slack resource。

`resources/slack.py` 负责构造 Slack client，属于外部服务适配层。它应基于 `dagster_slack.SlackResource` 实现，而不是重新发明 Slack 集成。推荐形态是：

```python
from dagster_slack import SlackResource
from slack_sdk.web.client import WebClient


class SlackAlertResource(SlackResource):
    channel_id: str
    http_proxy: str | None = None

    def get_client(self) -> WebClient:
        return WebClient(token=self.token, proxy=self.http_proxy)
```

这个类仍然是 Dagster Slack 官方 resource 的派生，只扩展 channel 和 proxy 两个项目配置。sensor 通过 resource 获取 client，并调用 `chat_postMessage(...)`。

当前项目已有两种配置模式：

1. `config/env.py`：集中声明 `dg.EnvVar` 常量，收敛直接读取环境变量的位置。
2. `resources/*.py`：`ConfigurableResource` 使用 `config/env.py` 中的 EnvVar 常量作为默认值，例如 `S3SettingsResource`。

Slack 应遵循同样模式：

```python
SLACK_BOT_TOKEN = dg.EnvVar("SLACK_BOT_TOKEN")
SLACK_CHANNEL_ID = dg.EnvVar("SLACK_CHANNEL_ID")
SLACK_HTTP_PROXY = dg.EnvVar("SLACK_HTTP_PROXY")
SLACK_WEBSERVER_BASE_URL = dg.EnvVar("DAGSTER_WEBSERVER_BASE_URL")
```

`SlackAlertResource` 使用这些 EnvVar 作为字段默认值；如需在单元测试中绕过环境变量，可直接构造 resource 传入普通字符串。

`webserver_base_url` 建议使用 `DAGSTER_WEBSERVER_BASE_URL`。如果暂不配置，则消息只包含 run id，不生成链接。

## 模块职责矩阵

| 文件 | 职责 | 约束 |
|------|------|------|
| `defs/config/env.py` | 声明 `SLACK_BOT_TOKEN`、`SLACK_CHANNEL_ID`、`SLACK_HTTP_PROXY`、可选 `DAGSTER_WEBSERVER_BASE_URL` | 只声明 EnvVar 或 required getter，不发送消息 |
| `defs/resources/slack.py` | 定义继承 `dagster_slack.SlackResource` 的 `SlackAlertResource` | 只负责 Slack client、channel、proxy、webserver URL 配置 |
| `defs/automation/slack_alerts.py` | 定义 `slack_asset_failure_sensor` 和消息构造纯函数 | 不导入任何 source 模块，不读取环境变量 |
| `defs/definitions.py` | 注册 `slack` resource 和 `slack_asset_failure_sensor` | 不包含消息格式细节 |
| `tests/unit/automation/test_slack_alerts.py` | 验证消息构造、截断、字段缺失处理 | 不真实调用 Slack |
| `tests/unit/resources/test_slack.py` | 验证 resource 继承、proxy 归一化、client 构造 | mock `WebClient` |
| `tests/integration/test_definitions_and_schedules.py` | 验证 definitions 基线变化 | 不依赖 Slack token 真值 |

## Definitions 装配

当前 `defs()` 只聚合 assets、jobs、schedules、resources。接入后应扩展为：

```python
return dg.Definitions(
    assets=bundle_assets(SOURCE_BUNDLES),
    jobs=bundle_jobs(SOURCE_BUNDLES),
    schedules=bundle_schedules(SOURCE_BUNDLES),
    sensors=[slack_asset_failure_sensor],
    resources={
        ...
        "slack": SlackAlertResource(),
    },
)
```

`SlackAlertResource()` 的字段默认值来自 `config/env.py`。这与现有 `S3SettingsResource()`、`ImageObjectStoreResource()` 的装配风格一致，顶层 definitions 不直接铺开环境变量细节。

sensor 不放入 `SourceBundle`，原因有三点：

1. `SourceBundle` 当前契约只有 assets、jobs、schedules；为一个全局告警扩展 bundle 契约会放大影响面。
2. Slack failure sensor 监听所有 source bundle 的 run failure，不属于某个数据源。
3. 告警注册应该随 repository 级 definitions 装配发生，与 source 增删解耦。

如果未来有更多跨源 sensors，可以在 `defs/automation/__init__.py` 或单独 helper 中提供 `AUTOMATION_SENSORS`，但不建议把 cross-source sensors 塞进各 source bundle。

## Sensor 方案

推荐 sensor 形态：

```python
@dg.run_failure_sensor(
    name="slack_asset_failure_sensor",
    default_status=dg.DefaultSensorStatus.RUNNING,
    minimum_interval_seconds=30,
)
def slack_asset_failure_sensor(
    context: dg.RunFailureSensorContext,
    slack: SlackAlertResource,
):
    ...
```

Dagster sensor 支持通过函数参数注入 resource，因此 sensor 应直接声明 `slack: SlackAlertResource`，而不是从全局变量或环境变量中构造 Slack client。

触发范围第一阶段不设置 `monitored_jobs`，即监听当前 repository 内所有 job failure。理由：

1. `SOURCE_BUNDLES` 下所有 job 都是需要运营关注的数据资产执行。
2. 新增 source bundle 后无需同步维护告警 job 白名单。
3. 当前 scheduler 没有明显不应告警的实验 job。

如果未来出现低优先级或测试 job，可改为从 `bundle_jobs(SOURCE_BUNDLES)` 生成 `monitored_jobs`，或者在 job tags 中增加 `slack_alert=false` 后由 sensor 过滤。

当前存在 `default_automation_condition_sensor`，用于驱动 automation condition。`slack_asset_failure_sensor` 与它职责不同：

| Sensor | 来源 | 职责 |
|--------|------|------|
| `default_automation_condition_sensor` | Dagster 根据 asset automation condition 生成 | 触发缺失或依赖更新后的资产执行 |
| `slack_asset_failure_sensor` | 本 RFC 新增 | 监听 run failure 并发送 Slack 通知 |

两者应并存。Slack sensor 不发起 `RunRequest`，只执行外部通知副作用。

## 失败资产识别

Dagster run failure sensor 的触发对象是 run，不是单个 asset。对于资产失败消息，建议按以下顺序提取上下文：

1. `context.get_step_failure_events()`：列出失败 step，取 `step_key` 和错误摘要。
2. `context.dagster_run.asset_selection`：列出本次 run 选择执行的资产集合，作为失败资产候选范围。
3. `context.partition_key`：提取分区 key；对日分区、年分区资产尤其重要。
4. `context.failure_event.message`：作为 run 级失败摘要。

第一阶段消息中使用“失败 step”和“资产候选”两个字段，而不是声称 100% 精确映射到唯一 asset。原因是 multi-asset、subset selection、资源初始化失败、IO manager 失败等场景可能没有单一 asset key。后续如需精确资产名，可增加 `AssetKey -> op name/step key` 的 definitions 索引，并为 multi-asset 建立显式映射。

结合当前资产形态，消息中的资产上下文应按以下规则展示：

| 当前场景 | 消息展示策略 |
|----------|--------------|
| 单 asset job，例如 `sina__trade_calendar_job` | `Assets` 通常展示唯一 asset key |
| 多 asset pipeline job，例如 `jiuyan__industry_ocr_pipeline_job` | `Assets` 展示 run 选择范围，`Failed steps` 指向实际失败 step |
| compaction automation 触发的 run | 展示 compacted asset key、partition key、失败 step |
| resource 初始化失败 | `Failed steps` 可能为空，展示 run 级 `Error` 和 job/run 信息 |
| IO manager 或 S3 写入失败 | `Failed steps` 指向执行 step，`Error` 保留存储错误摘要 |

## Slack 消息格式

建议使用 Slack Block Kit，并保留 `text` 作为移动端和通知 fallback。

Fallback text：

```text
Dagster asset run failed: {job_name} ({run_id})
```

Blocks 内容：

```text
Dagster asset run failed
Source: {source tags inferred from selected assets or "-"}
Job: {job_name}
Run: {run_id}
Partition: {partition_key or "-"}
Failed steps: {step_key list}
Assets: {asset_selection or "-"}
Error: {short_failure_message}
Open run: {webserver_run_url or "-"}
```

错误摘要需要截断，建议最多 1500 字符，避免 Slack 消息过长。完整堆栈仍以 Dagster UI 为准。

source 字段不应通过解析 job name 得出。优先从 selected asset 的 tags 中读取 `source`；如果当前 context 无法稳定拿到 asset definitions，则第一阶段可以省略 `Source` 字段，或只展示 asset key 中的 `source/<name>` 前缀信息。不要把 `eastmoney__`、`jiuyan__` 等命名约定写成强依赖。

## Proxy 设计

Slack Python SDK 的 `WebClient` 支持 `proxy` 参数；如果不传 `proxy`，也会尝试读取标准环境变量 `HTTPS_PROXY`、`https_proxy`、`HTTP_PROXY`、`http_proxy`。

本项目采用 `dagster_slack.SlackResource` 派生类里的显式 proxy：

```python
class SlackAlertResource(SlackResource):
    channel_id: str
    http_proxy: str

    def get_client(self) -> WebClient:
        proxy = self.http_proxy.strip() or None
        return WebClient(token=self.token, proxy=proxy)
```

理由：

1. `.env` 已使用 `SLACK_HTTP_PROXY` 表达 Slack 专用代理。
2. 显式传参不会影响已有 HTTP 数据源、BaoStock TCP、S3/RustFS 访问。
3. 测试时可以构造 fake resource，不需要依赖进程级代理环境。
4. 继续复用 `dagster-slack` 的 resource 抽象，后续若官方 `SlackResource` 增加 proxy 参数，可以移除项目级子类。

`SLACK_HTTP_PROXY` 应允许为空字符串。为空时传 `None` 给 Slack SDK，由部署环境自行决定是否通过标准代理变量出站。

部署层面不建议把 Slack proxy 配置到 `deploy/docker-compose.yml` 的全局服务环境中，除非 Dagster 进程也容器化并且所有出站请求都应走同一代理。当前 `deploy/docker-compose.yml` 只定义 RustFS、ClickHouse、PostgreSQL、NATS 等基础服务，没有 Dagster scheduler 服务；本地 `uv run dg dev` 或未来单独 scheduler 容器都应从根目录 `.env` 加载 Slack 变量。

## 依赖变更

`pipeline/scheduler/pyproject.toml` 需要新增：

```toml
dependencies = [
    ...
    "dagster-slack==0.29.6",
]
```

当前 scheduler 使用 `dagster==1.13.6`。Dagster integration libraries 的版本通常与 Dagster core 对应，`dagster==1.13.6` 对应 library 版本 `0.29.6`。如果升级 Dagster core，需要同步升级 `dagster-slack`。

不建议绕过 `dagster-slack` 直接依赖 `slack-sdk` 做完整自定义集成。`slack-sdk` 作为 `dagster-slack` 的传递依赖和底层 client 可以被使用，但 Dagster 侧的资源注册、生命周期和未来升级应以 `dagster_slack.SlackResource` 为边界。

## 环境变量变更

已存在并应继续使用：

```bash
SLACK_BOT_TOKEN=xoxb-...
SLACK_CHANNEL_ID=C0123456789
SLACK_HTTP_PROXY=http://proxy.example:7890
```

建议新增但非本次必须：

```bash
DAGSTER_WEBSERVER_BASE_URL=http://localhost:3000
```

生产环境应配置为可从 Slack 消息点击访问的 Dagster Webserver 地址。未配置时，消息不包含可点击 run URL。

`.env.example` 应同步列出 Slack 变量，但不能包含真实 token。示例：

```bash
SLACK_BOT_TOKEN=xoxb-your-bot-token
SLACK_CHANNEL_ID=C0123456789
SLACK_HTTP_PROXY=http://127.0.0.1:7890
DAGSTER_WEBSERVER_BASE_URL=http://localhost:3000
```

如果 `SLACK_HTTP_PROXY` 在某些环境不需要，应保留空值：

```bash
SLACK_HTTP_PROXY=
```

## 测试策略

实现时需要新增或调整以下测试：

1. `tests/unit/automation/test_slack_alerts.py`
   - 优先测试拆分出的纯消息构造函数，例如 `build_slack_failure_message(context_like)`，避免单元测试直接依赖复杂 Dagster context。
   - 使用 Dagster 官方测试 helper 覆盖最小 sensor 调用路径：`build_run_status_sensor_context(...)` + `RunStatusSensorContext.for_run_failure(...)`。
   - 验证 job、run、partition、step、asset selection、错误摘要格式。
   - 验证长错误消息会截断。
   - 验证 Slack 发送异常会被记录并吞掉，不从 sensor 继续抛出。
2. `tests/unit/resources/test_slack.py`
   - 验证 `SlackAlertResource` 字段默认值来自 `config/env.py`。
   - 验证 `SlackAlertResource` 是 `dagster_slack.SlackResource` 的派生类。
   - 验证 resource 构造 client 时传入 proxy。
   - 验证空字符串 proxy 会归一化为 `None`。
3. `tests/integration/test_definitions_and_schedules.py`
   - 扩展为 `test_registered_definitions_match_source_bundles_and_automation`。
   - 保持 source bundle 的 assets、jobs、schedules 断言不变。
   - 校验 `loaded_defs.sensors` 包含 `slack_asset_failure_sensor`。
   - 校验 resources 包含 `slack`。
4. `tests/integration/test_architecture_boundaries.py`
   - 保持 source 代码不直接读取 Slack 环境变量。
   - 如果新增边界测试，只允许 `config/` 和 `resources/slack.py` 读取 Slack 配置。

实现完成后，`dg list defs --json` 的关键验收应为：

```text
resources includes:
- slack: scheduler.defs.resources.slack.SlackAlertResource

sensors includes:
- default_automation_condition_sensor
- slack_asset_failure_sensor
```

`default_automation_condition_sensor` 不应消失；如果消失，说明 asset automation condition 注册被破坏。

验证命令仍遵循项目要求，在 `pipeline/` 下执行：

```bash
uv run ruff check scheduler/src scheduler/tests migrate
uv run ruff format scheduler/src scheduler/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests
uv run pytest scheduler/tests --cov=scheduler/src/scheduler --cov-report=term-missing
cd scheduler
uv run dg check defs
```

## 运行与运维

1. 本地开发时，`dg` 会从项目 `.env` 加载环境变量。
2. sensor 默认启动后，每次 run failure 会触发一次 Slack 发送。
3. Slack 发送失败只记录 sensor 日志，不应重新抛出为 sensor evaluation failure；原始 Dagster run 已经失败，告警失败不应制造第二个运维事件。
4. 如果 Slack API 返回 `channel_not_found`，优先检查 bot 是否已加入目标 channel，以及 `SLACK_CHANNEL_ID` 是否正确。
5. 如果出现连接超时，优先检查 `SLACK_HTTP_PROXY` 是否可从 Dagster 进程所在容器或主机访问。
6. 如果 Slack API 返回 `not_authed` 或 `invalid_auth`，优先检查 `SLACK_BOT_TOKEN` 是否是 bot token 且未过期。
7. 如果 Slack API 返回 `missing_scope`，需要给 Slack app 补充发送消息所需 scope，并重新安装 app。

建议本地验收流程：

1. `cd pipeline/scheduler && uv run dg list defs --json`，确认 resource 和 sensor 注册。
2. `cd pipeline/scheduler && uv run dg dev`，在 UI 中确认 `slack_asset_failure_sensor` 状态。
3. 使用一个不会写入真实生产数据的受控失败 job 或临时测试 job 验证 Slack 发送。
4. 验证完成后删除临时失败定义，不把测试 job 留在生产 definitions。

## 替代方案

### 方案 A：直接使用 `make_slack_on_run_failure_sensor`

优点：

- 实现最少。
- 官方 helper 已处理 run failure 消息。

缺点：

- 不直接支持 `SLACK_HTTP_PROXY` 专用变量。
- 不使用已注册的 Dagster `SlackResource`。
- 资产、分区、step failure 消息格式定制空间较小。
- 不利于在测试中验证 proxy 与消息构造。

结论：暂不采用，可作为后续简化方案。

如果未来 `dagster-slack` 的 helper 支持传入 `SlackResource` 或 proxy 参数，可以重新评估该方案。届时保留现有消息构造函数作为 `text_fn`/`blocks_fn`，只替换 sensor 外壳即可。

### 方案 B：使用 `slack_on_failure` hook

优点：

- 更接近 step failure，触发点更细。

缺点：

- 需要给每个 job 或 op/asset 绑定 hook，侵入现有 source bundle。
- 对新增数据源需要额外维护，容易遗漏。
- 资源初始化失败、run 级失败等场景覆盖不如 run failure sensor。

结论：不采用。

### 方案 C：Dagster Plus alert policy

优点：

- 可配置 asset materialization failure alert。
- 运维体验好，不需要自维护 sensor 代码。

缺点：

- 当前项目文档和部署形态以 OSS 本地 scheduler 为主。
- 不适合作为第一阶段默认方案。

结论：保留为未来迁移选项。

## 实施步骤

1. 在 `pipeline/scheduler/pyproject.toml` 添加 `dagster-slack` 依赖并同步 `uv.lock`。
2. 在 `defs/config/env.py` 添加 Slack EnvVar 常量。
3. 在 `defs/resources/slack.py` 添加继承 `dagster_slack.SlackResource` 的 `SlackAlertResource`，显式支持 `SLACK_HTTP_PROXY` 和可选 `DAGSTER_WEBSERVER_BASE_URL`。
4. 在 `defs/automation/slack_alerts.py` 添加：
   - failure context 到 Slack block 的纯函数；
   - `slack_asset_failure_sensor`。
5. 在 `defs/definitions.py` 注册 `slack` resource 和 `slack_asset_failure_sensor`，不修改 `SOURCE_BUNDLES`。
6. 同步 `.env.example` 的 Slack 变量占位。
7. 添加单元测试、definitions 集成测试和必要的边界测试。
8. 运行质量门禁和 `dg check defs`。
9. 在 Dagster UI 中确认 sensor 为 running，并用一个受控失败 run 验证 Slack 消息。

## 验收标准

1. `SOURCE_BUNDLES` 顺序和各 bundle 的 assets/jobs/schedules 不变。
2. `dg list defs --json` 中新增 `slack` resource 和 `slack_asset_failure_sensor`。
3. `default_automation_condition_sensor` 仍存在。
4. `slack_asset_failure_sensor` 不返回 `RunRequest`，只发送 Slack 消息。
5. Slack resource 继承 `dagster_slack.SlackResource`，不是完全自定义 Slack SDK 封装。
6. `SLACK_HTTP_PROXY` 只在 Slack resource 边界使用，不设置全局 `HTTP_PROXY`。
7. 单元测试覆盖消息构造、proxy 归一化、Slack 发送失败处理。
8. 质量门禁通过。

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| sensor 发送 Slack 失败 | 失败 run 没有告警 | 捕获异常并记录 sensor 日志；运维通过 sensor 日志排查 |
| 消息把多 asset run 误报为单 asset 失败 | 排障方向错误 | 第一阶段明确展示“资产候选”和 failed steps，不强行做唯一 asset 映射 |
| proxy 配置污染其他客户端 | S3、HTTP 数据源、BaoStock 行为变化 | 只把 `SLACK_HTTP_PROXY` 显式传给 Slack `WebClient` |
| helper API 后续变化 | 自定义 sensor 与官方 helper 偏离 | 保持资源基于 `dagster_slack.SlackResource`，消息构造函数独立，方便迁移 |
| `.env` 缺少 Slack 变量导致 definitions 加载失败 | 本地开发无法启动 Dagster | `.env.example` 给出占位；如希望 Slack 可选，后续可增加 `SLACK_ALERT_ENABLED` 开关 |

## 开放问题

1. 是否需要新增 `DAGSTER_WEBSERVER_BASE_URL`，用于 Slack 消息中的 run deeplink？
2. 是否需要按 source、asset tag 或 job tag 控制告警范围？
3. 是否需要为同一 run 的多次 sensor evaluation 做幂等防重？
4. 是否需要在后续支持 warning、数据为空、部分失败阈值触发等非 run failure 告警？
5. 是否需要 `SLACK_ALERT_ENABLED`，允许本地或 CI 加载 definitions 但不发送 Slack？

## 参考

- Dagster Slack integration：`dagster-slack` 提供 `SlackResource`、`make_slack_on_run_failure_sensor`、`slack_on_failure`。
- Dagster run failure sensor：`@run_failure_sensor` 可监听 run failure，并通过 `RunFailureSensorContext` 访问 run、failure event、step failure events。
- Slack Python SDK：`WebClient` 支持显式 `proxy` 参数，也可读取标准 `HTTP_PROXY/http_proxy/HTTPS_PROXY/https_proxy` 环境变量。
- 当前 scheduler 架构入口：`docs/architecture/scheduler-architecture.md`。
- 当前 module boundaries：`docs/architecture/scheduler-module-boundaries.md`。
