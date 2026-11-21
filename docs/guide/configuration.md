# 配置

Reader Next 通过环境变量进行配置。默认配置定义在 `src/app/config.rs` 中。

## 环境变量

### 服务器配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `SERVER_HOST` | `127.0.0.1` | 服务器绑定地址 |
| `SERVER_PORT` | `18080` | 服务器端口 |

### 数据库配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `DATABASE_URL` | `sqlite:storage/reader.db?mode=rwc` | SQLite 数据库连接字符串 |

### 存储配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `STORAGE_DIR` | `storage/` | 数据存储目录 |
| `ASSETS_DIR` | `storage/assets` | 资源文件目录 |

### Web 前端配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `WEB_ROOT` | `frontend/dist` | 静态 Web 文件目录 |

### 安全配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `SECURE` | `true` | 是否启用安全模式 |
| `SECURE_KEY` | 空 | 安全模式密钥 |

### 用户配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `INVITE_CODE` | 空 | 注册邀请码；配置时至少16字符，留空禁止开放注册 |
| `USER_LIMIT` | `50` | 最大用户数 |
| `USER_BOOK_LIMIT` | `2000` | 每用户最大书籍数 |

### 其他配置

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `LOG_LEVEL` | `info` | 日志级别 (trace, debug, info, warn, error) |
| `REQUEST_TIMEOUT_SECS` | `15` | HTTP 请求超时时间(秒) |

## 配置格式

环境变量使用 `__`（双下划线）作为分隔符表示层级结构。

例如，在嵌套配置中:
```bash
export APP__SERVER__PORT=3000
```

## 配置文件

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:210cefa651f4dd59a4e22b87bdac35c5db25ce2f0dda71bbbeab6c7b4b82be35 -->

你也可以创建 `.env` 文件。启动服务前，请生成一个至少 16 个字符的私密随机邀请码（例如运行 `openssl rand -hex 16`），并将生成的值填入 `INVITE_CODE`。不要留空，也不要复制公开示例中的值。如果你有意关闭安全模式下的公开注册，则可将邀请码留空。

```dotenv
SERVER_PORT=18080
DATABASE_URL=sqlite:storage/reader.db?mode=rwc
LOG_LEVEL=debug
SECURE=true
INVITE_CODE=
USER_LIMIT=50
USER_BOOK_LIMIT=2000
REQUEST_TIMEOUT_SECS=15
```

<!-- /coderabbit-ai-generated:v1 -->

## 开发配置

开发时建议:

```bash
LOG_LEVEL=debug
cargo run
```
