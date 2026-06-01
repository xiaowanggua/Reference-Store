[English](README.md)

# Refstore

面向 AI 使用的 Rust 命令行文献管理工具。配合 AI Skills，让 AI 能高效管理、检索和关联本地文献库。

## 特性

- 📄 **论文管理** — 手动添加、DOI/arXiv 自动抓取元数据、BibTeX 批量导入
- 🔍 **全文搜索** — 基于 SQLite FTS5，支持 grep 风格语法（AND/OR/NOT/短语/前缀）
- 🏷️ **标签系统** — 层级标签、标签别名、按标签筛选
- 📁 **分组系统** — 按个人用途归类论文（区别于标签）
- 📝 **笔记系统** — Markdown 笔记，支持多种类型（摘要/方法/结论/思考）
- 🔗 **引用关系** — 论文间引用关系图，支持 Mermaid 可视化
- 🤖 **AI 集成** — MCP Server (stdio) + AI Skills，可直接被 AI 工具调用
- 🌐 **Web 后台** — 轻量级管理界面，含引用关系图可视化（vis-network），支持中英文切换
- 💾 **备份恢复** — JSON 完整备份与恢复
- ⚡ **自动查重** — 添加论文时自动检测重复（arXiv ID > DOI > 标题）

## 安装

```bash
# 方式一：从 GitHub Releases 下载
# 1）下载对应系统的最新压缩包
# 2）解压后执行 setup：
./ref setup

# 构建
cargo build --release

# 安装到 ~/.refstore/bin/ 并注册 PATH
./target/release/ref setup
```

## 数据库配置

默认使用 `~/.refstore/refstore.db`（与安装目录同级）。可以通过以下方式指定：

```bash
# 通过 --db 参数（适用于所有命令）
ref --db /path/to/my-library.db list

# 通过 REFSTORE_DB 环境变量
export REFSTORE_DB=/path/to/my-library.db
ref list
```

## 快速开始

```bash
# 通过 DOI 添加论文（自动从 CrossRef 抓取元数据）
ref add --doi "10.1038/nature12373"

# 通过 arXiv ID 添加（自动从 arXiv API 抓取元数据）
ref add --arxiv 2301.07041

# 手动添加（指定所有字段）
ref add --title "My Paper" --authors "Alice,Bob" --url "https://..." --venue "NeurIPS 2024"

# 强制添加（跳过查重）
ref add --title "Existing Paper" --arxiv 2301.07041 --force

# 列出所有论文（默认：表格格式，每页 10 条）
ref list

# 带筛选条件和分页
ref list --tag NLP --status unread --sort date --page 2 --page-size 20

# 按条件统计数量
ref count --tag "Deep Learning" --status unread

# grep 风格搜索
ref search "deep learning"          # AND: 两个词同时出现
ref search '"attention mechanism"'  # 精确短语
ref search "transformer OR attention"  # OR: 任一词
ref search "deep -learning"         # 排除
ref search "transform*"              # 前缀通配符

# 查看论文详情（支持短 ID）
ref get <paper-id>
ref get <paper-id> --format json

# 更新论文字段
ref update <paper-id> --title "新标题" --read

# 删除论文
ref delete <paper-id>

# 给论文打标签
ref tag add <paper-id> NLP

# 层级标签和别名
ref tag add <paper-id> BERT --parent NLP --alias "Bidirectional Encoder"

# 添加笔记
ref note add <paper-id> --content "关键发现：..." --note-type summary

# 搜索笔记
ref note search "transformer" --page 1 --page-size 5

# 创建引用关系
ref cite add <from-id> <to-id> --relation cites --strength 4 --note "第3节对此有扩展"

# 查看引用关系图
ref cite graph <paper-id> --depth 3 --format mermaid

# 列出某篇论文的所有引用
ref cite list <paper-id>

# 完整 JSON 备份
ref backup papers-backup.json

# 从备份恢复（合并到现有数据库）
ref import papers-backup.json

# 从 BibTeX 文件导入
ref import references.bib

# 导出论文
ref export --format bibtex
ref export --format markdown --tag "NLP"
ref export --format mermaid --graph <paper-id>

# 查看统计（总数、已读/未读、月度趋势、标签云）
ref stats

# 启动 Web 管理后台（http://localhost:8080）
ref serve

# 启动 MCP Server（供 AI 工具调用）
ref mcp
```

## 搜索语法

搜索命令支持 grep 风格的查询语法：

| 查询 | 含义 |
|------|------|
| `deep learning` | AND — 匹配同时包含两个词的论文 |
| `"attention mechanism"` | 精确短语匹配 |
| `transform*` | 前缀通配符 |
| `deep -learning` | 排除 — 包含 "deep" 但不含 "learning" |
| `deep NOT learning` | 排除（另一种写法） |
| `transformer OR attention` | OR — 匹配任一词 |
| `BERT` | 单词前缀匹配（默认行为） |

搜索还支持按标签、分组过滤，以及限定搜索范围：

```bash
ref search "transformer" --tag NLP --group survey
ref search "neural" --in title
```

## 命令一览

| 命令 | 说明 |
|------|------|
| `ref add` | 添加论文（--doi / --arxiv / 手动字段，--force 跳过查重） |
| `ref get <id>` | 查看详情（--format text/json，支持短 ID） |
| `ref list` | 分页列表（--tag / --group / --status / --sort / --format / --page） |
| `ref count` | 按条件计数（--tag / --group / --status） |
| `ref search <query>` | 全文搜索，grep 风格语法（--in / --tag / --group） |
| `ref update <id>` | 更新字段（--read / --unread / --title / --doi 等） |
| `ref delete <id>` | 删除论文 |
| `ref tag add <id> <name>` | 打标签（--parent 层级，--alias 别名） |
| `ref tag remove <id> <name>` | 移除标签 |
| `ref tag list` | 列出所有标签（树形结构） |
| `ref tag delete <name>` | 删除标签 |
| `ref note add <id>` | 添加笔记（--content, --note-type: summary/method/result/thought/general） |
| `ref note list <id>` | 列出论文笔记 |
| `ref note search <keyword>` | 搜索笔记（--page, --page-size） |
| `ref group add <name>` | 创建分组（--description） |
| `ref group assign <id> <group>` | 加入分组（自动创建分组） |
| `ref group list` | 列出所有分组及论文数 |
| `ref cite add <from> <to>` | 添加引用（--relation, --strength 1-5, --note） |
| `ref cite graph <id>` | 引用关系图（--depth, --format text/mermaid） |
| `ref import <file>` | 导入（.bib BibTeX / .json 备份） |
| `ref export` | 导出（--format bibtex/markdown/mermaid/json, --tag, --id, --graph） |
| `ref backup <file>` | 完整 JSON 备份 |
| `ref stats` | 统计概览（总数、已读/未读、月度趋势、标签云） |
| `ref serve` | 启动 Web 后台（http://localhost:8080，支持中英文切换） |
| `ref mcp` | 启动 MCP Server（stdio 模式） |
| `ref setup` | 安装并注册 PATH |

## 输出格式

支持三种输出格式（适用于 `list`、`search` 等命令）：

- **table**（默认）— 格式化表格，人类友好
- **compact** — 编号列表，仅 ID + 标题
- **json** — JSON 数组，适合管道/脚本处理

```bash
ref list --format json
ref list --format compact --page-size 20
```

## Web 管理后台

通过 `ref serve` 启动，运行在 http://localhost:8080，提供：

- 论文列表（分页、搜索）
- 论文详情（元数据、笔记、引用关系）
- 已读/未读切换、删除
- 标签和分组管理
- 引用关系图可视化（vis-network）
- **中英文切换**：点击导航栏的"中文"或"EN"即可切换语言，偏好保存在 cookie 中

## 项目结构

```
papermanager/
├── crates/
│   ├── core/          # 核心库：数据模型、SQLite 存储、搜索、抓取
│   ├── cli/           # CLI 入口（ref 命令）
│   ├── server/        # Web 管理后台（Axum + Askama）
│   └── mcp/           # MCP Server（JSON-RPC over stdio）
├── skills/            # AI Skills（通用 Markdown 指令）
├── DESIGN.md          # 详细设计文档
├── README.md          # 英文 README
└── README_zh.md       # 本文件
```

## 技术栈

| 层面 | 选型 |
|------|------|
| 语言 | Rust (edition 2021) |
| CLI | clap v4 (derive) |
| 数据库 | SQLite (rusqlite) + FTS5 |
| HTTP | axum |
| 模板 | askama（编译时检查） |
| MCP | JSON-RPC over stdio |
| 序列化 | serde + serde_json |

## AI 集成

### MCP Server

```bash
# 在 AI 工具中配置 MCP Server
ref mcp
```

提供的 MCP 工具：
- `paper_add` / `paper_list` / `paper_get` / `paper_search`
- `paper_tag` / `paper_note` / `paper_cite` / `paper_count`

### AI Skills

`skills/` 目录包含通用 Markdown 指令文档，描述遇到某类用户需求时执行什么 CLI 命令。

| Skill | 职责 |
|-------|------|
| paper-add | 通过 DOI/arXiv/手动方式添加论文，处理查重 |
| paper-search | grep 风格搜索、过滤、分页、计数 |
| paper-graph | 管理引用关系、可视化关系图 |
| paper-notes | 添加、搜索和管理分类笔记 |

## License

MIT
