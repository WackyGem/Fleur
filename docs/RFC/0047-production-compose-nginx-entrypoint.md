# RFC 0047: 生产 Docker Compose 与 nginx 统一入口

状态：Implemented
日期：2026-07-03
领域：deploy-ops, racingline, dagster, rearview
关联系统：deploy/, app/racingline/, pipeline/scheduler/, engines/
相关文档：
- docs/architecture/deploy-ops.md
- docs/architecture/racingline.md
- docs/architecture/scheduler-architecture.md
- docs/architecture/rearview.md
- deploy/docker-compose.yml
- .env.example
- docs/plans/archive/0076-production-compose-nginx-implementation-plan.md
- docs/jobs/reports/2026-07-03-production-compose-nginx-smoke.md

## 摘要

当前 [deploy/docker-compose.yml](../../deploy/docker-compose.yml) 是开发和 smoke run 使用的基础设施入口，只定义 RustFS、ClickHouse、PostgreSQL 和 NATS。Racingline、Rearview 和 Dagster Web UI 仍由本机进程通过 `make racingline-dev`、`make webui` 等命令启动。

本 RFC 建议把现有 compose 明确改名为开发配置：

```text
deploy/docker-compose.dev.yaml
```

并重新设计 [deploy/docker-compose.yml](../../deploy/docker-compose.yml) 作为生产配置。生产配置继续复用 dev 中相同的基础设施参数、环境变量、健康检查、资源限制和 volume 语义，但新增由 `build` 构建的应用镜像，并引入 nginx 作为统一 HTTP 入口：

1. `/` 默认服务 Racingline。
2. `/dagster/` 代理 Dagster Web UI。
3. `/rearview/` 作为 Racingline 调用 Rearview API 的同源后端路径。

Compose 文件中应用服务使用 `build`，不依赖本机 `cargo run`、`npm run dev` 或 `uv run dg dev`。Docker Compose 官方示例支持在 service 中使用 `build`，并可通过 `depends_on.condition: service_healthy` 等待依赖服务健康后启动上游服务；生产 compose 应采用这种显式依赖关系。

## 目标

1. 保留现有开发体验：当前基础设施 compose 原样迁移为 `deploy/docker-compose.dev.yaml`，`make dev-up`、`make racingline-dev` 等开发命令改为使用 dev compose。
2. 将 `deploy/docker-compose.yml` 改为 production-like 入口，包含基础设施、Rearview server、Rearview worker、Dagster Web UI、Dagster daemon 和 nginx gateway。
3. 生产配置中的应用服务必须通过 Dockerfile `build` 构建镜像。
4. nginx 是唯一默认对外 HTTP 入口，访问根路径进入 Racingline。
5. 基础设施服务参数与 dev 保持一致，避免 dev/prod 两套数据库、对象存储和消息队列语义漂移。
6. dev compose 的宿主机端口使用 `34xxx` 命名空间，production-like compose 的宿主机端口使用 `35xxx` 命名空间，避免两套环境在同一台机器上互相抢占端口。
7. 生产入口支持最小可验证的启动、健康检查和浏览器访问 smoke。

## 非目标

1. 本 RFC 不引入 Kubernetes、Swarm、Nomad、Traefik 或云托管服务。
2. 本 RFC 不新增认证、用户隔离、TLS 证书自动签发或公网安全模型。
3. 本 RFC 不改变 Dagster asset graph、dbt 模型、Rearview API 或 Racingline 页面行为。
4. 本 RFC 不改变 PostgreSQL migration 的权威来源；迁移仍由 `pipeline/migrate` 管理。
5. 本 RFC 不要求第一版移除基础设施端口暴露；端口收敛可作为后续 hardening。

## 当前事实

### Compose

当前 [deploy/docker-compose.yml](../../deploy/docker-compose.yml) 包含：

| Service | 角色 | 当前暴露端口 |
|---|---|---|
| `rustfs` | S3-compatible object storage | `${RUSTFS_API_PORT:-34050}:9000`, `${RUSTFS_CONSOLE_PORT:-34051}:9001` |
| `rustfs-init` | bucket 初始化 | 不暴露 |
| `clickhouse` | ClickHouse server | `${CLICKHOUSE_HTTP_PORT:-34052}:8123`, `${CLICKHOUSE_NATIVE_PORT:-34053}:9000` |
| `postgres` | PostgreSQL | `${POSTGRES_PORT:-34054}:5432` |
| `nats` | NATS JetStream | `${NATS_CLIENT_PORT:-34055}:4222`, `${NATS_MONITOR_PORT:-34056}:8222` |

该文件没有应用镜像、Dockerfile、nginx 配置、Dagster Web UI service 或 Rearview service。

### 本地运行入口

当前开发入口在 [Makefile](../../Makefile) 中：

| Target | 当前行为 |
|---|---|
| `dev-up` | `docker compose --env-file .env -f deploy/docker-compose.yml up -d` |
| `racingline-dev` | 启动基础设施，执行 migration/catalog sync，再以本机进程启动 Rearview server、worker 和 Vite dev server |
| `webui` | 在 `pipeline/scheduler` 下执行 `uv run dg dev --host ... --port ...` |

### 应用边界

Racingline 位于 [app/racingline/](../../app/racingline/)，当前通过 Vite 构建静态产物：

```bash
cd app/racingline
npm run build
```

Rearview 位于 [engines/](../../engines/)，当前服务进程为：

```bash
cd engines
cargo run -p rearview-server -- serve
cargo run -p rearview-portfolio-worker -- run
```

Dagster scheduler 位于 [pipeline/scheduler/](../../pipeline/scheduler/)，当前开发 Web UI 通过 `dg dev` 启动。生产镜像需要明确采用 `dagster-webserver` + `dagster-daemon`，或保留 `dg dev` 作为短期 production-like 过渡命令；最终生产配置不应依赖本机 `make webui`。

## 目标拓扑

```text
Client
  |
  v
nginx :${FLEUR_HTTP_PORT:-35080}
  |-- /                         -> Racingline static files
  |-- /assets/*                 -> Racingline static assets
  |-- /rearview/*               -> rearview-server:34057
  |-- /dagster/*                -> dagster-webserver:3000
  |
  +-- internal network: fleur
        |-- rearview-server
        |-- rearview-portfolio-worker
        |-- dagster-webserver
        |-- dagster-daemon
        |-- postgres
        |-- clickhouse
        |-- rustfs
        |-- nats
```

nginx 只暴露统一 HTTP 入口。基础设施端口第一版可继续按 dev 参数暴露，便于 migration、dbt、ClickHouse smoke 和人工排障；生产 hardening 阶段再评估是否把 PostgreSQL、ClickHouse、NATS 和 RustFS console 限制为内网或 ops-only profile。

## 宿主机端口命名空间

dev 和 production-like compose 必须使用不同宿主机端口段：

1. dev compose 使用 `34xxx` 宿主机端口，保留当前本地开发和 smoke run 习惯。
2. production-like compose 使用 `35xxx` 宿主机端口，允许在同一台开发机或测试机上与 dev compose 并存。
3. 容器内端口保持服务原生端口，不随宿主机端口段变化。
4. `.env` 中如果显式覆盖端口，必须遵守当前 compose 文件对应的端口段；更推荐后续拆分 `.env.dev` 和 `.env.prod`，避免一个 `.env` 同时承载两套宿主机端口。

端口矩阵：

| 服务 | 容器端口 | dev 宿主机端口 | production-like 宿主机端口 |
|---|---:|---:|---:|
| RustFS API | `9000` | `34050` | `35050` |
| RustFS console | `9001` | `34051` | `35051` |
| ClickHouse HTTP | `8123` | `34052` | `35052` |
| ClickHouse native | `9000` | `34053` | `35053` |
| PostgreSQL | `5432` | `34054` | `35054` |
| NATS client | `4222` | `34055` | `35055` |
| NATS monitor | `8222` | `34056` | `35056` |
| Rearview HTTP | `34057` | `34057` | `35057`（仅排障或显式暴露时使用） |
| nginx unified HTTP | `80` | 不适用 | `35080` |

Racingline Vite dev server 当前仍由 [Makefile](../../Makefile) 使用 `5173` 启动；它不是 dev compose 基础设施端口。若后续把 dev Racingline 也纳入 compose，再单独分配 `34xxx` 段内端口。

## 文件职责

### `deploy/docker-compose.dev.yaml`

开发 compose 由当前 [deploy/docker-compose.yml](../../deploy/docker-compose.yml) 重命名得到，服务内容保持不变。

开发命令同步改为：

```makefile
COMPOSE_DEV_FILE := deploy/docker-compose.dev.yaml
```

并让以下 target 使用 dev compose：

```text
dev-up
dev-down
dev-logs
rearview-prepare
racingline-dev
```

### `deploy/docker-compose.yml`

生产 compose 成为默认 compose 文件，包含：

| Service | 镜像来源 | 角色 |
|---|---|---|
| `nginx` | `build` | 统一入口；默认服务 Racingline；代理 `/dagster/` 和 `/rearview/` |
| `rearview-server` | `build` | Rearview HTTP API |
| `rearview-portfolio-worker` | `build` | Portfolio/backtest 异步任务 worker |
| `dagster-webserver` | `build` | Dagster Web UI |
| `dagster-daemon` | `build` | Dagster schedules/sensors/daemon 运行面 |
| `db-migrate` | `build` | one-shot PostgreSQL Alembic migration |
| `rearview-catalog-sync` | `build` | one-shot Rearview metric catalog sync |
| `rustfs` | image | 与 dev 相同 |
| `rustfs-init` | image | 与 dev 相同 |
| `clickhouse` | image | 与 dev 相同 |
| `postgres` | image | 与 dev 相同 |
| `nats` | image | 与 dev 相同 |

应用服务使用 `depends_on` 等待基础设施健康：

| Service | 依赖 |
|---|---|
| `db-migrate` | `postgres` |
| `rearview-catalog-sync` | `db-migrate`, `clickhouse` |
| `rearview-server` | `rearview-catalog-sync`, `nats` |
| `rearview-portfolio-worker` | `rearview-catalog-sync`, `nats`, `rearview-server` |
| `dagster-webserver` | `db-migrate`, `clickhouse`, `rustfs`, `rustfs-init` |
| `dagster-daemon` | `db-migrate`, `clickhouse`, `rustfs`, `rustfs-init`, `dagster-webserver` |
| `nginx` | `rearview-server`, `dagster-webserver` |

## 镜像构建

### nginx gateway

新增：

```text
deploy/nginx/default.conf
deploy/docker/nginx/Dockerfile
```

推荐 gateway 镜像采用 multi-stage build：

1. Node build stage：在 `app/racingline` 执行 `npm ci` 和 `npm run build`。
2. nginx runtime stage：复制 Racingline `dist/` 到 nginx html root，并复制 `deploy/nginx/default.conf`。

这样 `nginx` service 既是统一入口，也是 Racingline 静态资源宿主，不需要额外的 Vite preview 或 Node runtime。

生产构建时，Racingline API base 应为空字符串，让现有 API client 中以 `/rearview/...` 开头的请求走 nginx 同源代理：

```text
VITE_REARVIEW_API_BASE_URL=
```

### Rearview

新增：

```text
deploy/docker/rearview/Dockerfile
```

推荐使用 Rust multi-stage build：

1. builder stage 在 `engines/` 下构建 release 二进制：
   - `rearview-server`
   - `rearview-portfolio-worker`
2. runtime stage 复制二进制和 `engines/crates/rearview-core/config/metric_policy.yml`。
3. `rearview-server` 和 `rearview-portfolio-worker` 使用同一个镜像，通过不同 `command` 启动。

容器内环境变量应使用 Docker 网络名，而不是宿主端口：

```text
REARVIEW_HTTP_BIND=0.0.0.0:34057
REARVIEW_DATABASE_URL=postgresql://fleur:...@postgres:5432/rearview
REARVIEW_NATS_URL=nats://nats:4222
CLICKHOUSE_HOST=clickhouse
CLICKHOUSE_PORT=8123
```

### Dagster

新增：

```text
deploy/docker/dagster/Dockerfile
```

Dagster 镜像从 `pipeline/` 构建，安装 uv workspace 依赖，并包含 scheduler project、contract tools、elt dbt project、contracts、migrate 和必要配置。

第一版需要在实现前明确 Dagster 运行命令：

| 方案 | 命令 | 适用性 |
|---|---|---|
| 推荐生产态 | `dagster-webserver` + `dagster-daemon` | 更接近长期生产运行面 |
| 短期过渡 | `uv run dg dev --host 0.0.0.0 --port 3000` | 简单，但语义仍偏开发 |

如果采用推荐生产态，应把当前只在 `scheduler` dev dependency group 中的 `dagster-webserver` 调整为镜像构建可安装的运行依赖，或在 Dockerfile 中显式安装包含 webserver 的依赖组。不能让生产 compose 依赖宿主机已有的 uv 环境。

## 启动初始化链路

生产镜像服务拉起后应自动执行 PostgreSQL migration 和 Rearview catalog sync，但不应把这两类初始化逻辑放进 `rearview-server`、`dagster-webserver` 或 `dagster-daemon` 的 entrypoint。推荐使用 Compose one-shot init services 表达初始化边界：

```text
postgres/clickhouse/rustfs/nats healthy
  -> db-migrate completed successfully
  -> rearview-catalog-sync completed successfully
  -> rearview-server / rearview-portfolio-worker / dagster-webserver / dagster-daemon
  -> nginx
```

### `db-migrate`

`db-migrate` 使用 Dagster/pipeline 镜像执行 Alembic，因为 migration 权威入口在 [pipeline/migrate/](../../pipeline/migrate/)。

命令：

```bash
cd pipeline
uv run alembic -c migrate/alembic.ini -x target=all upgrade head
```

容器化后等价为：

```yaml
db-migrate:
  build:
    context: ..
    dockerfile: deploy/docker/dagster/Dockerfile
  command:
    - uv
    - run
    - alembic
    - -c
    - migrate/alembic.ini
    - -x
    - target=all
    - upgrade
    - head
  restart: "no"
  depends_on:
    postgres:
      condition: service_healthy
  environment:
    PIPELINE_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/pipeline
    REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
```

`target=all` 是当前 [Makefile](../../Makefile) 中 `rearview-migrate` 的事实入口。失败时必须阻断后续应用服务启动，避免 schema 未到位时启动 Rearview、Dagster 或 worker。

### `rearview-catalog-sync`

`rearview-catalog-sync` 使用 Rearview 镜像执行 metric catalog 同步：

```bash
cd engines
cargo run -p rearview-server -- catalog sync
```

容器化后等价为：

```yaml
rearview-catalog-sync:
  build:
    context: ..
    dockerfile: deploy/docker/rearview/Dockerfile
  command: ["rearview-server", "catalog", "sync"]
  restart: "no"
  depends_on:
    db-migrate:
      condition: service_completed_successfully
    clickhouse:
      condition: service_healthy
  environment:
    REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
    CLICKHOUSE_HOST: clickhouse
    CLICKHOUSE_PORT: 8123
    CLICKHOUSE_USER: ${CLICKHOUSE_USER}
    CLICKHOUSE_PASSWORD: ${CLICKHOUSE_PASSWORD}
    REARVIEW_CLICKHOUSE_MARTS_DATABASE: ${REARVIEW_CLICKHOUSE_MARTS_DATABASE}
```

`catalog sync` 必须保持幂等；实现阶段如果发现当前 sync 不是事务性 upsert，应先补齐 Rearview 侧幂等约束，再把它纳入生产自动启动链路。

### 启动约束

1. `db-migrate` 和 `rearview-catalog-sync` 都是 one-shot service，设置 `restart: "no"`。
2. 长期服务通过 `depends_on.condition: service_completed_successfully` 依赖 init job，不在自己的 entrypoint 中重复执行 migration 或 catalog sync。
3. init job 失败时让 compose 启动失败；不要让 nginx 对外暴露半初始化应用。
4. 生产 release script 或 `make prod-up` 可以显式执行 init job，但 compose 依赖仍应保留，防止直接 `docker compose up` 绕过初始化。
5. PostgreSQL DDL migration 和 Rearview catalog sync 不应由健康检查触发；健康检查只能读取状态，不执行写入。

推荐受控启动命令：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d --build postgres clickhouse rustfs nats
docker compose --env-file .env -f deploy/docker-compose.yml run --rm db-migrate
docker compose --env-file .env -f deploy/docker-compose.yml run --rm rearview-catalog-sync
docker compose --env-file .env -f deploy/docker-compose.yml up -d --build --wait
```

同时保留直接启动路径：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d --build --wait
```

直接启动路径必须通过 `depends_on` 表达同样的顺序。

## nginx 路由契约

nginx 第一版路由：

| 路径 | 上游 | 说明 |
|---|---|---|
| `/` | static files | Racingline 默认入口，SPA fallback 到 `index.html` |
| `/assets/` | static files | Vite assets，长缓存 |
| `/rearview/` | `http://rearview-server:34057/rearview/` | Racingline API，同源转发 |
| `/dagster/` | `http://dagster-webserver:3000/` | Dagster Web UI |

`/dagster/` 是主要风险点。Dagster Web UI 如果生成绝对路径、WebSocket 地址或静态资源路径时不支持 path prefix，第一版实现必须通过浏览器 smoke 验证。若 path prefix 不稳定，允许在同一个 nginx gateway 内保留默认 `/` 给 Racingline，并把 Dagster 改为 host-based route：

```text
http://fleur.local/          -> Racingline
http://dagster.fleur.local/  -> Dagster
```

该 fallback 不改变“统一 nginx 入口”和“默认 Racingline”的目标，只把 Dagster 从 path route 改为 server_name route。

## 生产 compose 草图

以下是目标结构草图，不是最终可直接复制的完整文件：

```yaml
services:
  db-migrate:
    build:
      context: ..
      dockerfile: deploy/docker/dagster/Dockerfile
    command:
      - uv
      - run
      - alembic
      - -c
      - migrate/alembic.ini
      - -x
      - target=all
      - upgrade
      - head
    restart: "no"
    environment:
      PIPELINE_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/pipeline
      REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
    depends_on:
      postgres:
        condition: service_healthy
    networks:
      - fleur

  rearview-catalog-sync:
    build:
      context: ..
      dockerfile: deploy/docker/rearview/Dockerfile
    command: ["rearview-server", "catalog", "sync"]
    restart: "no"
    environment:
      REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
      CLICKHOUSE_HOST: clickhouse
      CLICKHOUSE_PORT: 8123
      CLICKHOUSE_USER: ${CLICKHOUSE_USER}
      CLICKHOUSE_PASSWORD: ${CLICKHOUSE_PASSWORD}
      REARVIEW_CLICKHOUSE_MARTS_DATABASE: ${REARVIEW_CLICKHOUSE_MARTS_DATABASE}
    depends_on:
      db-migrate:
        condition: service_completed_successfully
      clickhouse:
        condition: service_healthy
    networks:
      - fleur

  nginx:
    build:
      context: ..
      dockerfile: deploy/docker/nginx/Dockerfile
      args:
        VITE_REARVIEW_API_BASE_URL: ""
    ports:
      - "${FLEUR_HTTP_PORT:-35080}:80"
    depends_on:
      rearview-server:
        condition: service_healthy
      dagster-webserver:
        condition: service_started
    networks:
      - fleur

  rearview-server:
    build:
      context: ..
      dockerfile: deploy/docker/rearview/Dockerfile
    command: ["rearview-server", "serve"]
    environment:
      REARVIEW_HTTP_BIND: 0.0.0.0:34057
      REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
      REARVIEW_NATS_URL: nats://nats:4222
      CLICKHOUSE_HOST: clickhouse
      CLICKHOUSE_PORT: 8123
      CLICKHOUSE_USER: ${CLICKHOUSE_USER}
      CLICKHOUSE_PASSWORD: ${CLICKHOUSE_PASSWORD}
      REARVIEW_CLICKHOUSE_MARTS_DATABASE: ${REARVIEW_CLICKHOUSE_MARTS_DATABASE}
    expose:
      - "34057"
    depends_on:
      rearview-catalog-sync:
        condition: service_completed_successfully
      nats:
        condition: service_healthy
    networks:
      - fleur

  rearview-portfolio-worker:
    build:
      context: ..
      dockerfile: deploy/docker/rearview/Dockerfile
    command: ["rearview-portfolio-worker", "run"]
    environment:
      REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
      REARVIEW_NATS_URL: nats://nats:4222
      CLICKHOUSE_HOST: clickhouse
      CLICKHOUSE_PORT: 8123
      CLICKHOUSE_USER: ${CLICKHOUSE_USER}
      CLICKHOUSE_PASSWORD: ${CLICKHOUSE_PASSWORD}
    depends_on:
      rearview-catalog-sync:
        condition: service_completed_successfully
      rearview-server:
        condition: service_healthy
    networks:
      - fleur

  dagster-webserver:
    build:
      context: ..
      dockerfile: deploy/docker/dagster/Dockerfile
    command: ["dagster-webserver", "-h", "0.0.0.0", "-p", "3000"]
    environment:
      DAGSTER_HOME: /opt/dagster/dagster_home
      DAGSTER_WEBSERVER_BASE_URL: http://localhost:${FLEUR_HTTP_PORT:-35080}/dagster
      PIPELINE_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/pipeline
      REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
      RUSTFS_ENDPOINT: http://rustfs:9000
      CLICKHOUSE_HOST: clickhouse
      CLICKHOUSE_PORT: 8123
      REARVIEW_NATS_URL: nats://nats:4222
    depends_on:
      db-migrate:
        condition: service_completed_successfully
      clickhouse:
        condition: service_healthy
      rustfs:
        condition: service_healthy
    networks:
      - fleur

  dagster-daemon:
    build:
      context: ..
      dockerfile: deploy/docker/dagster/Dockerfile
    command: ["dagster-daemon", "run"]
    environment:
      DAGSTER_HOME: /opt/dagster/dagster_home
      PIPELINE_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/pipeline
      REARVIEW_DATABASE_URL: postgresql://${POSTGRES_USER}:${POSTGRES_PASSWORD}@postgres:5432/rearview
      RUSTFS_ENDPOINT: http://rustfs:9000
      CLICKHOUSE_HOST: clickhouse
      CLICKHOUSE_PORT: 8123
      REARVIEW_NATS_URL: nats://nats:4222
    depends_on:
      db-migrate:
        condition: service_completed_successfully
      clickhouse:
        condition: service_healthy
      rustfs:
        condition: service_healthy
      dagster-webserver:
        condition: service_started
    networks:
      - fleur
```

基础设施 service 块从 dev compose 复制并保持参数一致。实现时不要抽象出不透明的 YAML 生成脚本；第一版保持两个 compose 文件直读可审查。

## 实施步骤

1. 重命名当前 `deploy/docker-compose.yml` 为 `deploy/docker-compose.dev.yaml`。
2. 更新 `Makefile` 和 `docs/architecture/deploy-ops.md` 中所有开发命令引用。
3. 新增生产 `deploy/docker-compose.yml`，先复制 dev 基础设施 service，再追加应用 service。
4. 新增 nginx gateway Dockerfile 和 nginx 配置。
5. 新增 Rearview Dockerfile，确保 `rearview-server` 和 `rearview-portfolio-worker` 可从同一镜像启动。
6. 新增 Dagster Dockerfile，明确 `dagster-webserver` 和 `dagster-daemon` 生产命令。
7. 新增 `db-migrate` one-shot service，使用 Dagster/pipeline 镜像执行 `uv run alembic -c migrate/alembic.ini -x target=all upgrade head`。
8. 新增 `rearview-catalog-sync` one-shot service，使用 Rearview 镜像执行 `rearview-server catalog sync`，并让应用服务依赖其成功完成。
9. 更新 `.env.example`，补充 `FLEUR_HTTP_PORT=35080`、production-like `35xxx` 宿主机端口和容器内生产变量说明。
10. 增加 smoke run 文档或 job report，记录生产 compose 首次启动结果。

## 验收标准

文档和配置层面：

```bash
make docs-check
git diff --check
docker compose --env-file .env -f deploy/docker-compose.dev.yaml config
docker compose --env-file .env -f deploy/docker-compose.yml config
```

镜像构建：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml build nginx rearview-server dagster-webserver db-migrate rearview-catalog-sync
```

初始化链路：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml run --rm db-migrate
docker compose --env-file .env -f deploy/docker-compose.yml run --rm rearview-catalog-sync
```

生产 smoke：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/rearview/health
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/dagster/
```

Racingline browser smoke：

1. 访问 `http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/`。
2. 确认首屏加载 Racingline，而不是 Dagster。
3. 确认 Racingline API 请求走同源 `/rearview/`。
4. 访问 `/dagster/`，确认 Dagster UI 静态资源、GraphQL 请求和 WebSocket 请求没有路径前缀错误。

如果 `/dagster/` path prefix 验证失败，本 RFC 允许切换为同一 nginx 下的 host-based route，并把失败原因记录到 job report。

## 风险与待决问题

1. Dagster Web UI 的 path prefix 兼容性需要真实浏览器验证；这是本 RFC 最大不确定性。
2. Dagster production command 需要在实现阶段确认 workspace/code location 文件和 dependency group 边界，避免把 `dg dev` 当成长期生产入口。
3. `rearview-server catalog sync` 必须先确认幂等和事务边界；如果 sync 中途失败，不应留下与代码版本不一致的半同步 catalog。
4. 第一版继续暴露基础设施端口有利于迁移和排障，但不等同于公网安全配置。
5. `.env` 同时服务 dev 和 production-like compose 时，需要清晰区分宿主访问地址与容器内服务地址；后续应评估拆分 `.env.dev` 和 `.env.prod`，防止 `34xxx` / `35xxx` 端口覆盖互相污染。
