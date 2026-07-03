# Prod Docker Example Fixture v2 Deploy

日期：2026-07-03

范围：将 `example__portfolio_live_job` 相关的 `racingline_0051_low_reversal` / `v2` example fixture 改动发布到 production-like Docker 栈。

## 结论

需要重新 build 应用镜像。原因：

- `engines/crates/rearview-core/src/examples.rs` 被编译进 `rearview-server`，只重启旧容器不会加载新的 `v2` fixture。
- `pipeline/scheduler/src/scheduler/defs/rearview/assets.py` 被打进 Dagster 镜像，`example__portfolio_live_job` 的 expected version 从 `v1` 改为 `v2`，需要重建 Dagster webserver/daemon 镜像。

本次没有重建 nginx、PostgreSQL、ClickHouse、NATS 或 RustFS。

## 执行命令

```bash
docker compose --env-file .env -f deploy/docker-compose.yml build rearview-server dagster-webserver

docker compose --env-file .env -f deploy/docker-compose.yml up -d --no-deps --force-recreate rearview-server
docker compose --env-file .env -f deploy/docker-compose.yml up -d --no-deps --force-recreate rearview-portfolio-worker
docker compose --env-file .env -f deploy/docker-compose.yml up -d --no-deps --force-recreate dagster-webserver
docker compose --env-file .env -f deploy/docker-compose.yml up -d --no-deps --force-recreate dagster-daemon
```

构建出的镜像：

| 镜像 | digest | created |
|---|---|---|
| `fleur/rearview:local` | `sha256:fcc2231888be72996bcaef9aebe41077cf66fd1f547e479ae39eab111b4b3044` | `2026-07-03T16:08:53.628847839Z` |
| `fleur/dagster:local` | `sha256:e206ac1bc30c03f4cf872cd615b6d1b593d0ddd3232caa01f49ca8cdbd3e5417` | `2026-07-03T16:08:00.879214625Z` |

重建容器：

- `fleur-prod-rearview-server`
- `fleur-prod-rearview-portfolio-worker`
- `fleur-prod-dagster-webserver`
- `fleur-prod-dagster-daemon`

## 验证

已执行：

```bash
docker compose --env-file .env -f deploy/docker-compose.yml ps
curl -fsS --noproxy '*' http://127.0.0.1:35080/rearview/health
python3 - <<'PY'
import urllib.request
for url in ['http://127.0.0.1:35080/dagster/', 'http://127.0.0.1:35080/rearview/version']:
    with urllib.request.urlopen(url, timeout=10) as response:
        print(url, response.status)
PY
docker exec fleur-prod-dagster-webserver python -c "from scheduler.defs.rearview.assets import EXAMPLE_0051_VERSION; print(EXAMPLE_0051_VERSION)"
docker exec fleur-prod-rearview-server sh -c "grep -a -o -F 'Racingline Strategy Search Low Reversal Example' /usr/local/bin/rearview-server | head -n 1"
```

结果：

- `rearview-server` health：`healthy`，nginx `/rearview/health` 返回 `{"status":"ok","component":"rearview-server","version":"0.1.0"}`。
- Dagster webserver health：`healthy`，nginx `/dagster/` 返回 `200`。
- `rearview-portfolio-worker` 启动并连接 NATS，日志显示 schema check 成功。
- `dagster-daemon` 启动，日志显示 daemons loaded。
- Dagster 容器内 `EXAMPLE_0051_VERSION` 输出 `v2`。
- Rearview server 二进制包含 `Racingline Strategy Search Low Reversal Example`。

未执行 `POST /rearview/examples/strategy-portfolios/racingline-0051-low-reversal/ensure` 或 `dg launch --job example__portfolio_live_job`，避免在发布验证阶段主动创建或复用 prod example portfolio。
