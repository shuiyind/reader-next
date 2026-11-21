# 快速开始

## 环境要求

- Rust stable
- Node.js 20+
- npm

## 克隆项目

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:629c9dba99af20df55c4f67c25e630b7a564c83b9fe35cb9ae3e35d77a0b7abd -->

```bash
git clone https://github.com/shuiyind/reader-next.git
cd reader-next
```

<!-- /coderabbit-ai-generated:v1 -->

## 推荐启动方式

Reader Next 本地默认用单端口模式：先构建前端，再让 Rust 服务端提供页面和 API。

```bash
cd frontend
npm install
npm run build

cd ..
SERVER_PORT=18080 cargo run
```

打开：

```text
http://localhost:18080
```

后端 API 前缀为 `/reader3`。

## 前端热更新

只有需要调试前端时才运行 Vite：

```bash
cd frontend
npm run dev
```

Vite 默认地址：

```text
http://localhost:5173
```

开发环境会把 `/reader3` 代理到本地后端。后端端口变化时，要同步检查 Vite 代理配置。

## 运行测试

```bash
cargo test

cd frontend
npm run test
```

## 配置

配置从 `.env` 或环境变量读取。复制 `.env.example` 后按需修改：

```bash
cp .env.example .env
```

常用配置：

| 配置项 | 默认值 | 说明 |
| --- | --- | --- |
| `SERVER_HOST` | `127.0.0.1` | 服务绑定地址 |
| `SERVER_PORT` | `18080` | 服务端口 |
| `DATABASE_URL` | `sqlite:storage/reader.db?mode=rwc` | SQLite 数据库 |
| `WEB_ROOT` | `frontend/dist` | 前端静态资源目录 |
| `LOG_LEVEL` | `info` | 日志级别 |

更多配置见 [配置指南](./configuration)。

## 文档站

```bash
cd docs
npm install
npm run docs:dev
```

线上地址：

```text
https://maple0517.github.io/reader-next/
```
