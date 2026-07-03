# Plan 0076: Production Compose 与 nginx 统一入口实施计划

日期：2026-07-03

状态：Completed

领域：deploy-ops, Docker, Racingline, Rearview, Dagster

关联系统：

- `deploy/`
- `app/racingline/`
- `engines/`
- `pipeline/`
- `Makefile`
- `.env.example`

关联文档：

- [RFC 0047: 生产 Docker Compose 与 nginx 统一入口](../../RFC/0047-production-compose-nginx-entrypoint.md)
- [Deploy And Ops Architecture](../../architecture/deploy-ops.md)
- [Racingline Architecture](../../architecture/racingline.md)
- [Rearview Architecture](../../architecture/rearview.md)
- [Scheduler Architecture](../../architecture/scheduler-architecture.md)
- [2026-07-03 Production Compose nginx Smoke](../../jobs/reports/2026-07-03-production-compose-nginx-smoke.md)

## 背景

RFC 0047 已确定部署方向：

1. 现有 `deploy/docker-compose.yml` 改为 `deploy/docker-compose.dev.yaml`，继续服务本地开发和 smoke run。
2. 新的 `deploy/docker-compose.yml` 作为 production-like compose，基础设施参数与 dev 保持一致，但宿主机端口使用 `35xxx`。
3. 应用服务通过 Dockerfile `build` 构建镜像。
4. nginx 是统一 HTTP 入口，默认进入 Racingline，`/rearview/` 代理 Rearview API，`/dagster/` 代理 Dagster Web UI。
5. PostgreSQL migration 和 Rearview metric catalog sync 由 one-shot init services 自动执行，并阻断未初始化完成的应用服务启动。

RFC 0047 编写时，仓库没有应用 Dockerfile、nginx 配置或 production compose；当时 `deploy/docker-compose.yml` 只定义 RustFS、ClickHouse、PostgreSQL 和 NATS。开发入口由 `Makefile` 通过本机 `cargo run`、`npm run dev` 和 `uv run dg dev` 组合启动。

Docker 官方文档对本计划相关的构建原则是：使用 multi-stage build 减少最终镜像内容；先复制 lockfile/manifest 安装依赖，再复制源代码，以提升依赖层缓存命中率。

## 目标

1. 完成 dev/prod compose 分离，并保持现有开发命令可用。
2. 建立 production-like compose，默认 `docker compose -f deploy/docker-compose.yml up` 能拉起基础设施、初始化任务、应用服务和 nginx。
3. 建立 nginx、Rearview、Dagster 三类 Dockerfile，采用可审查、可缓存、运行镜像尽量小的构建流程。
4. 解决 dev `34xxx` 与 production-like `35xxx` 宿主机端口隔离，避免 `.env` 中 dev 端口污染生产 compose。
5. 自动执行 Alembic migration 和 Rearview catalog sync，失败时阻断应用和 nginx 对外服务。
6. 补齐 Makefile 生产入口、配置检查、构建检查和 smoke run 命令。
7. 更新 `.env.example`、deploy architecture 和运行报告，使后续维护有事实入口。

## 非目标

1. 不引入 Kubernetes、Swarm、Nomad、Traefik、外部 registry 或云发布平台。
2. 不新增认证、TLS、域名证书、公网访问策略或 WAF。
3. 不改变 Dagster asset graph、dbt 模型、Rearview API 或 Racingline 用户流程。
4. 不在应用服务 entrypoint 中隐式执行 migration 或 catalog sync。
5. 不把 Docker build 优化扩展成全仓库重构；第一版只优化生产镜像所需路径。
6. 不要求第一版移除基础设施宿主机端口暴露；生产 hardening 后续单独处理。

## 实施前事实基线

| 区域 | 事实 |
|---|---|
| Compose | RFC 0047 编写前，`deploy/docker-compose.yml` 只含 RustFS、ClickHouse、PostgreSQL、NATS。 |
| Dockerfile | RFC 0047 编写前，仓库没有应用 Dockerfile，也没有 `.dockerignore`。 |
| Racingline | `app/racingline` 使用 Vite，构建命令为 `npm run build`，lockfile 为 `package-lock.json`。 |
| Rearview | `engines` 是 Cargo workspace，目标二进制为 `rearview-server` 和 `rearview-portfolio-worker`。 |
| Dagster | scheduler 在 `pipeline/scheduler`，`dagster-webserver` 当前在 scheduler dev dependency group。 |
| Migration | `Makefile` 的 `rearview-migrate` 使用 `cd pipeline && uv run alembic -c migrate/alembic.ini -x target=all upgrade head`。 |
| Catalog sync | `Makefile` 的 `rearview-catalog-sync` 使用 `cd engines && cargo run -p rearview-server -- catalog sync`。 |
| 端口 | dev 使用 `34xxx`；production-like 需要使用 `35xxx`，nginx 默认 `35080`。 |

## 设计约束

1. production compose 的宿主机端口变量不要复用 dev 端口变量名。若直接使用 `${RUSTFS_API_PORT:-35050}`，根目录 `.env` 中已有 `34050` 会污染生产端口。生产 compose 应使用 `FLEUR_PROD_*` 前缀端口变量，例如 `FLEUR_PROD_RUSTFS_API_PORT:-35050`。
2. 容器内连接一律使用 Docker service name 和容器端口，例如 `postgres:5432`、`clickhouse:8123`、`nats:4222`、`rustfs:9000`。
3. `db-migrate` 和 `rearview-catalog-sync` 是 one-shot services，`restart: "no"`，长期服务通过 `depends_on.condition: service_completed_successfully` 等待它们。
4. Dockerfile 不复制 `node_modules`、`dist`、`target`、`.venv`、`.dagster`、dbt target/logs 或 `.git` 到 build context。
5. 第一版 Dockerfile 优先使用稳定、可读的 multi-stage build。只有在构建时间确认不可接受时，再引入更复杂的 `cargo-chef` 或远程 cache。
6. 新增构建脚本或 Make target 必须是 thin wrapper，不能隐藏 compose 文件、环境文件和服务名。

## Dockerfile 构建策略

### nginx / Racingline

目标：nginx 镜像同时作为 gateway 和 Racingline 静态资源宿主。

构建策略：

1. 使用 Node builder stage。
2. 先复制 `app/racingline/package.json` 和 `app/racingline/package-lock.json`，执行 `npm ci`。
3. 再复制 `app/racingline` 源码，执行 `npm run build`。
4. runtime stage 使用 nginx image，复制 `dist/` 到 nginx html root。
5. 复制 `deploy/nginx/default.conf`。
6. build arg 固定支持 `VITE_REARVIEW_API_BASE_URL=""`，让现有 `/rearview/...` API paths 走 nginx 同源代理。

完成标准：

1. `docker compose ... build nginx` 不依赖本机 `node_modules`。
2. 修改 React 源码不应导致 `npm ci` 层失效。
3. 运行镜像中不包含源码、lockfile 或 dev dependency cache。

### Rearview

目标：一个 Rearview 镜像提供 `rearview-server` 和 `rearview-portfolio-worker` 两个二进制。

构建策略：

1. 使用 Rust builder stage 和 slim runtime stage。
2. 先复制 `engines/Cargo.toml`、`engines/Cargo.lock` 和各 crate `Cargo.toml`，执行依赖预取或最小 cache 预热。
3. 由于当前 Rust workspace 使用 `crates/*` 成员匹配，`cargo fetch` 需要看到 crate target 文件；第一版允许在 builder stage 中先复制完整 `engines/` 再 fetch/build，保证构建可用。
4. 在同一 build step 中构建并复制二进制到 `/out`，避免 target cache mount 产物无法进入最终镜像。
5. runtime stage 只复制：
   - `rearview-server`
   - `rearview-portfolio-worker`
   - `engines/crates/rearview-core/config/metric_policy.yml`
   - 必要 CA certificates
6. 如果使用 BuildKit cache mount，只缓存 Cargo registry/git；不要把最终需要复制的 target 目录只留在 cache mount 中。

完成标准：

1. `rearview-server --version` 和 `rearview-portfolio-worker --version` 可在容器内执行。
2. `rearview-server catalog sync` 可在同一镜像中执行。
3. 运行镜像不包含完整 Rust source tree、Cargo registry 或 target build cache。

### Dagster / Pipeline

目标：一个 pipeline/Dagster 镜像支持 `db-migrate`、`dagster-webserver` 和 `dagster-daemon`。

构建策略：

1. 使用 Python 3.12 runtime，Python 版本与 `pipeline/.python-version` 保持一致。
2. 先复制 `pipeline/pyproject.toml`、`pipeline/uv.lock`、workspace member pyproject，再安装依赖。
3. 再复制 `pipeline/scheduler`、`pipeline/contract_tools`、`pipeline/elt`、`pipeline/contracts`、`pipeline/migrate`。
4. 生产镜像不能依赖宿主 uv 环境。
5. `dagster-webserver` 当前在 scheduler dev dependency group；实施时必须二选一：
   - 将 `dagster-webserver` 调整为部署可安装依赖组。
   - 或在 Dockerfile 中显式同步包含 webserver 的 dependency group，并在文档中说明这是 production image 的运行依赖。
6. 镜像内 `DAGSTER_HOME` 使用容器路径，例如 `/opt/dagster/dagster_home`。
7. Dagster 镜像若需要构建 Furnace CLI，沿用 Rearview 的 Rust workspace 约束：第一版以可构建为优先，后续再评估 `cargo-chef` 或 registry cache 优化。

完成标准：

1. 容器内可执行 `uv run alembic -c migrate/alembic.ini -x target=all upgrade head`。
2. 容器内可执行 `dagster-webserver` 和 `dagster-daemon`。
3. 镜像不复制本地 `.venv`、`.dagster`、dbt `target/` 或 logs。

## 实施阶段

### Phase 1: Compose 文件切分与开发入口保持

目标：先把 dev/prod 文件边界切开，不引入应用服务。

实施项：

1. 将当前 `deploy/docker-compose.yml` 移动为 `deploy/docker-compose.dev.yaml`。
2. 更新 `Makefile`：
   - `COMPOSE_DEV_FILE := deploy/docker-compose.dev.yaml`
   - `dev-up`、`dev-down`、`dev-logs` 使用 dev compose。
   - `rearview-prepare`、`racingline-dev` 继续走 dev compose。
3. 新增空壳或基础设施版 `deploy/docker-compose.yml`，先复制 dev 基础设施 service，再切换宿主机端口到 `35xxx`。
4. 更新 `docs/architecture/deploy-ops.md` 的开发命令路径。

完成标准：

1. `docker compose --env-file .env -f deploy/docker-compose.dev.yaml config` 通过。
2. `make dev-up` 仍启动原开发基础设施。
3. `docker compose --env-file .env -f deploy/docker-compose.yml config` 通过，且宿主机端口为 `35xxx`。

### Phase 2: 环境变量与端口治理

目标：避免 dev `.env` 中的 `34xxx` 端口污染 production-like compose。

实施项：

1. production compose 的 host port mapping 使用 `FLEUR_PROD_*` 变量：
   - `FLEUR_PROD_RUSTFS_API_PORT:-35050`
   - `FLEUR_PROD_RUSTFS_CONSOLE_PORT:-35051`
   - `FLEUR_PROD_CLICKHOUSE_HTTP_PORT:-35052`
   - `FLEUR_PROD_CLICKHOUSE_NATIVE_PORT:-35053`
   - `FLEUR_PROD_POSTGRES_PORT:-35054`
   - `FLEUR_PROD_NATS_CLIENT_PORT:-35055`
   - `FLEUR_PROD_NATS_MONITOR_PORT:-35056`
   - `FLEUR_PROD_REARVIEW_HTTP_PORT:-35057`，仅排障 profile 或显式暴露时使用
   - `FLEUR_HTTP_PORT:-35080`
2. `.env.example` 保留 dev 变量，并新增 production-like 端口变量块。
3. production service environment 中的连接地址显式使用容器内地址，不读取 dev host port：
   - `PIPELINE_DATABASE_URL=postgresql://...@postgres:5432/pipeline`
   - `REARVIEW_DATABASE_URL=postgresql://...@postgres:5432/rearview`
   - `CLICKHOUSE_HOST=clickhouse`
   - `CLICKHOUSE_PORT=8123`
   - `REARVIEW_NATS_URL=nats://nats:4222`
   - `RUSTFS_ENDPOINT=http://rustfs:9000`

完成标准：

1. `docker compose ... config` 展开后生产宿主机端口全部在 `35xxx` 或 `35080`。
2. 生产容器内连接不出现 `127.0.0.1:34xxx`。
3. `.env.example` 明确 dev/prod 端口职责。

### Phase 3: Build Context 与 `.dockerignore`

目标：先控制 build context，避免 Docker build 上传无关大目录。

实施项：

1. 新增根目录 `.dockerignore`。
2. 排除：
   - `.git`
   - `.dagster`
   - `.venv`、`pipeline/.venv`
   - `app/racingline/node_modules`
   - `app/racingline/dist`
   - `engines/target`
   - `pipeline/elt/target`
   - `pipeline/elt/logs`
   - Python cache、pytest cache、ruff cache
3. 保留 Dockerfile 必需的 lockfile、manifest、源码、migrate、contracts 和 deploy 配置。
4. production compose 的 build context 使用仓库根目录，Dockerfile 通过 `dockerfile: deploy/docker/.../Dockerfile` 指定。

完成标准：

1. `docker compose ... build --progress=plain nginx` 不向 build context 发送 `node_modules`、`dist` 或 `engines/target`。
2. 三个 Dockerfile 都能从同一个 repo root context 读取所需文件。

### Phase 4: nginx/Racingline 镜像与路由

目标：构建统一入口镜像并验证默认 Racingline。

实施项：

1. 新增 `deploy/docker/nginx/Dockerfile`。
2. 新增 `deploy/nginx/default.conf`。
3. nginx 配置：
   - `/` serve Racingline static，SPA fallback 到 `index.html`。
   - `/assets/` 使用静态资源缓存。
   - `/rearview/` proxy 到 `rearview-server:34057/rearview/`。
   - `/dagster/` proxy 到 `dagster-webserver:3000/`。
   - WebSocket headers 覆盖 Dagster 场景。
4. Racingline build arg 设置 `VITE_REARVIEW_API_BASE_URL=""`，避免前端请求变成 `/rearview/rearview/...`。

完成标准：

1. `docker compose ... build nginx` 通过。
2. 访问 `http://127.0.0.1:35080/` 返回 Racingline `index.html`。
3. Racingline 请求路径使用 `/rearview/`，不再指向 `127.0.0.1:34057`。

### Phase 5: Rearview 镜像与 one-shot catalog sync

目标：Rearview 服务和 worker 容器化，并复用同一镜像执行 catalog sync。

实施项：

1. 新增 `deploy/docker/rearview/Dockerfile`。
2. 构建 `rearview-server` 和 `rearview-portfolio-worker` release binary。
3. 新增 production compose services：
   - `rearview-catalog-sync`
   - `rearview-server`
   - `rearview-portfolio-worker`
4. `rearview-catalog-sync` 依赖 `db-migrate` 成功和 ClickHouse healthy。
5. `rearview-server` 和 worker 依赖 `rearview-catalog-sync` 成功。
6. 确认或补齐 Rearview HTTP health endpoint；如果当前没有稳定 health endpoint，先新增只读 health route，再用于 compose healthcheck 和 smoke。

完成标准：

1. `docker compose ... build rearview-server rearview-portfolio-worker rearview-catalog-sync` 通过。
2. `docker compose ... run --rm rearview-catalog-sync` 成功且可重复运行。
3. `rearview-server` healthcheck 成功。
4. worker 启动后不因 NATS/ClickHouse/PostgreSQL 容器地址错误退出。

### Phase 6: Dagster 镜像与 db-migrate

目标：Dagster webserver/daemon 和 Alembic migration 容器化。

实施项：

1. 新增 `deploy/docker/dagster/Dockerfile`。
2. 调整 scheduler dependency group，使生产镜像能安装 `dagster-webserver`。
3. 新增 production compose services：
   - `db-migrate`
   - `dagster-webserver`
   - `dagster-daemon`
4. `db-migrate` 执行 `uv run alembic -c migrate/alembic.ini -x target=all upgrade head`。
5. `dagster-webserver` 和 `dagster-daemon` 依赖 `db-migrate` 成功。
6. 明确 Dagster code location/workspace 启动方式；禁止生产 compose 长期使用宿主 `make webui`。

完成标准：

1. `docker compose ... build dagster-webserver dagster-daemon db-migrate` 通过。
2. `docker compose ... run --rm db-migrate` 成功且可重复运行。
3. `dagster-webserver` 能加载 scheduler definitions。
4. `dagster-daemon` 能启动且不因 `DAGSTER_HOME` 或 workspace 路径错误退出。

### Phase 7: Production Compose 聚合与 Makefile 入口

目标：把所有服务串成可操作入口。

实施项：

1. 新增 Make targets：
   - `prod-config`
   - `prod-build`
   - `prod-up`
   - `prod-down`
   - `prod-logs`
   - `prod-init`
2. `prod-init` 显式执行：
   - `docker compose ... run --rm db-migrate`
   - `docker compose ... run --rm rearview-catalog-sync`
3. `prod-up` 可采用受控路径：
   - 启动基础设施
   - 执行 init
   - `up -d --build --wait`
4. 保留 compose 级 `depends_on.condition: service_completed_successfully`，防止绕过 `prod-init`。
5. 文档中明确 `make racingline-dev` 仍是开发入口，`make prod-up` 是 production-like smoke 入口。

完成标准：

1. `make prod-config` 通过。
2. `make prod-build` 能构建应用镜像。
3. `make prod-up` 能按顺序拉起 init job、应用服务和 nginx。
4. `make dev-up` 和 `make racingline-dev` 行为不退化。

### Phase 8: Smoke、浏览器验收与运行报告

目标：用真实容器验证 RFC 0047 端到端。

实施项：

1. 运行 production compose smoke：
   - `curl http://127.0.0.1:35080/`
   - `curl http://127.0.0.1:35080/rearview/health`
   - `curl http://127.0.0.1:35080/dagster/`
2. 使用 Playwright CDP 验证：
   - 默认首屏为 Racingline。
   - Racingline network 请求走 `/rearview/`。
   - `/dagster/` 静态资源、GraphQL 和 WebSocket 没有 path prefix 错误。
3. 如果 `/dagster/` path prefix 失败，按 RFC 0047 切换为同一 nginx 下的 host-based route，并在报告中记录失败证据。
4. 新增运行报告到 `docs/jobs/reports/`。
5. 更新 `docs/architecture/deploy-ops.md`，记录新入口和端口分层。

完成标准：

1. production compose 端到端 smoke 通过。
2. 浏览器截图或 DOM/network 证据说明默认入口是 Racingline。
3. 运行报告记录命令、时间、端口、服务状态和任何保留风险。

## 最小验证命令

文档与静态检查：

```bash
make docs-check
git diff --check
```

Compose 配置：

```bash
docker compose --env-file .env -f deploy/docker-compose.dev.yaml config
docker compose --env-file .env -f deploy/docker-compose.yml config
```

应用镜像构建：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml build nginx rearview-server rearview-portfolio-worker dagster-webserver dagster-daemon db-migrate rearview-catalog-sync
```

初始化：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml run --rm db-migrate
docker compose --env-file .env -f deploy/docker-compose.yml run --rm rearview-catalog-sync
```

生产 smoke：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml up -d --build --wait
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/rearview/health
curl -fsS http://127.0.0.1:${FLEUR_HTTP_PORT:-35080}/dagster/
```

领域门禁：

```bash
cd app/racingline
npm run lint
npm run typecheck
npm test
npm run build

cd ../../engines
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd ../pipeline
uv run ruff check scheduler/src scheduler/tests contract_tools/src contract_tools/tests migrate
uv run pyright scheduler/src/scheduler scheduler/tests contract_tools/src/fleur_contracts contract_tools/tests
uv run pytest scheduler/tests contract_tools/tests --cov=scheduler/src/scheduler --cov=contract_tools/src/fleur_contracts --cov-report=term-missing
cd scheduler
uv run dg check defs
```

## 禁止模式

1. 不在 `rearview-server`、`dagster-webserver` 或 `dagster-daemon` entrypoint 中执行 migration 或 catalog sync。
2. 不让 production compose 读取 dev host port 变量作为宿主机端口映射。
3. 不把 `node_modules`、`engines/target`、`.venv` 或 `.dagster` 复制进 Docker build context。
4. 不通过本机进程补齐 production compose 中缺失的服务；production smoke 必须使用容器服务。
5. 不用 healthcheck 执行写入操作。
6. 不在 nginx 中把 Dagster 设为默认根路径；根路径必须服务 Racingline。

## 完成标准

1. `deploy/docker-compose.dev.yaml` 保留现有开发基础设施语义，开发命令可用。
2. `deploy/docker-compose.yml` 能构建并启动 production-like 栈，使用 `35xxx` 宿主机端口。
3. nginx 默认入口服务 Racingline，`/rearview/` 和 `/dagster/` 路由通过 smoke。
4. `db-migrate` 和 `rearview-catalog-sync` 自动执行且失败阻断后续服务。
5. Dockerfile 构建层具备基础缓存策略，运行镜像不包含明显无关大目录。
6. Makefile、`.env.example`、deploy architecture 和运行报告同步更新。
7. 本计划完成后移入 `docs/plans/archive/`，并在 `docs/plans/README.md` 最近完成区域登记。
