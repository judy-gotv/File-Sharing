# 文件分享站 (FileShare)

Rust + Axum + SQLite 实现的轻量文件/链接分享列表站。访客可以搜索、按分类筛选、翻页、一键复制地址、点击名字下载；管理员通过带侧边栏的后台维护数据、上传文件、查看仪表盘和统计、管理安全设置。

## 安装（Docker，推荐）

```bash
# 拉取镜像
docker pull ghcr.io/judy-gotv/file-sharing:latest

# 启动（数据保存在具名卷 fileshare-data，删容器不丢）
docker run -d --name fileshare \
  -p 8080:8080 \
  -e ADMIN_USER=admin \
  -e ADMIN_PASSWORD=请改成至少8位的强密码 \
  -e TRUST_PROXY=true \
  -v /opt/fileshare-data:/app/data \
  ghcr.io/judy-gotv/file-sharing:latest
```

> 上传文件的地址会自动跟随访问方式：IP+端口访问就显示 `http://IP:端口/files/...`，
> 经反代域名访问就显示 `https://域名/files/...`（反代需透传 `Host` 与 `X-Forwarded-Proto`，
> 并设置 `TRUST_PROXY=true`）。`PUBLIC_BASE_URL` 仅在取不到 Host 时作为兜底。

启动后打开：

- 前台展示：http://服务器IP:8080/
- 后台管理：http://服务器IP:8080/admin（首次用 `ADMIN_USER` / `ADMIN_PASSWORD` 登录）

> 镜像由 GitHub Actions 自动构建并发布到 GitHub Container Registry，
> 每次推送到 `main` 分支会自动更新 `latest` 标签；
> 打 `v1.2.3` 这样的 tag 会额外发布 `1.2.3` / `1.2` 标签。
> 镜像包页面：https://github.com/judy-gotv/File-Sharing/pkgs/container/file-sharing

也可用 `docker compose`（见 `docker-compose.yml`，先按 `.env.example` 建好 `.env`）：

```bash
docker compose up -d --build
```

## 功能

- 两套 UI 可切换：原生深色 / SaaS 浅色，另有 5 种主题色，右上角切换
- 全站多语言：简体中文 / 繁體中文 / English，右上角切换
- 列表展示：要求标签（必须/可选）、名字、地址、说明、下载次数、失效标记
- 搜索 / 分类筛选 / 分页（后端查询）
- 一键复制地址（上报复制次数）、下载统计（`/go/:id` 跳转计数）
- 管理后台：仪表盘、条目管理、分类管理、统计、安全（SVG 侧边栏导航）
- 账号密码登录（Argon2 哈希、7 天会话）、登录限速 + IP 黑名单
- 拖拽排序 / 批量删除 / 批量移动分类
- 失效链接检测（后台定时 + 手动触发）
- 文件上传：本地目录或 S3 / Cloudflare R2
- `/healthz` 健康检查、`/metrics` Prometheus 指标
- M3U 文件作为普通条目添加即可，系统不做解析

## 从源码构建

```bash
cargo build --release
ADMIN_PASSWORD=至少8位 ./target/release/fileshare
```

环境变量见 `.env.example`（`LISTEN`、`DATABASE_URL`、`TRUST_PROXY`、`PUBLIC_BASE_URL`、`UPLOAD_DIR`、`UPLOAD_MAX_MB`、`CHECK_INTERVAL_MIN`、`METRICS_TOKEN`、S3 相关）。

## API 速览

- `GET /api/items?q=&category=&page=1&size=20` 列表/搜索/筛选/分页
- `GET /go/:id` 下载计数并 307 跳转
- `POST /api/auth/login` → `{"token","username","role"}`，管理接口需 `Authorization: Bearer <token>`
- `GET /api/admin/dashboard` 仪表盘数据
- `POST /api/admin/upload` 上传文件（multipart，字段名 `file`）

完整接口说明见开发文档。
