# PaperManager — 项目规划文档

## 一、项目定位

面向 AI 使用的 Rust 命令行文献管理工具，配合通用 AI Skills，让 AI 能高效管理、检索和关联本地文献库。

附带一个轻量管理后台网页，供人类查看和维护文献数据。

**核心原则**：CLI 和 Skills 是主体，Web 后台是辅助。

---

## 二、功能规划

### 2.1 文献存储

- 存储论文基本信息：标题、作者、摘要、原文链接
- 支持 DOI、arXiv ID 关联
- 支持本地 PDF 文件路径关联
- 记录发表日期、期刊/会议（venue）
- 记录创建和更新时间

### 2.2 导入

- 手动添加（指定标题、URL 等字段）
- 通过 DOI 自动抓取元数据（CrossRef API）
- 通过 arXiv ID 自动抓取元数据
- BibTeX 文件批量导入
- 从 URL 自动提取标题和摘要

### 2.2.1 自动查重

添加论文时自动检测重复，防止 AI 或人工重复插入相同论文：
- 通过 arXiv ID 查重（arXiv ID 全局唯一，是最可靠的查重依据）
- 通过 DOI 查重
- 通过标题模糊匹配查重（作为兜底）
- 检测到重复时返回已有论文信息，不重复插入
- 支持强制插入（`--force`）跳过查重

### 2.3 标签系统

- 给论文打标签
- 层级标签（支持父子关系，如 `ML > DL > Transformer`）
- 标签别名/同义词（如 `LLM` = `Large Language Model`）
- 列出所有标签（树形展示）
- 按标签筛选论文

### 2.4 分组系统

- 给论文归入分组（如"毕业论文参考"、"项目 A 文献"、"每日阅读"）
- 一篇论文可属于多个分组
- 分组与标签的区别：标签是描述论文属性的（NLP、Transformer），分组是按个人用途归类的（毕业论文、项目A）
- 列出所有分组及其论文数量
- 按分组筛选论文列表

### 2.5 笔记系统

- 给论文添加 Markdown 格式笔记
- 支持多种笔记类型：摘要、方法、结论、个人思考、通用
- 按关键词搜索笔记内容
- 查看某篇论文的所有笔记

### 2.6 阅读状态

- 仅两种状态：已读 / 未读
- 按状态筛选论文列表

### 2.7 引用关系

- 手动标记论文间引用关系
- 支持多种关系类型：引用（cites）、相关（related）、对比（contrasts）、扩展（extends）、改进（improves）
- 关系强度权重（1-5）
- 查询某篇论文的关系子图（指定深度）
- 导出关系图为 Mermaid 格式

### 2.8 搜索

- 关键词搜索（指定范围：标题、摘要、笔记、全局）
- 标签组合过滤（AND 逻辑）
- 按分组过滤
- 按状态过滤
- 按时间、标题排序
- 模糊搜索（fuzzy matching）
- 分页返回结果（每页默认 10 条，防止 AI 上下文爆）

### 2.9 列表与分页

- 分页展示论文列表，每页默认 10 条，可配置
- 支持多种输出格式：文本表格（默认）、JSON（可选，用于管道/脚本）、精简（compact，仅 ID+标题）
- 支持按标签、分组、状态、排序方式筛选
- 独立的 `count` 命令，仅返回匹配数量（节省上下文）

### 2.10 导出

- 导出为 BibTeX 格式
- 导出为 Markdown 格式
- 导出关系图为 Mermaid 格式
- JSON 完整备份与恢复

### 2.11 统计

- 总论文数、已读/未读数
- 标签分布（标签云）
- 月度新增趋势

### 2.12 AI 集成

- 默认输出人类和 AI 都可直接阅读的格式化文本（对齐、缩进、清晰的结构）
- JSON 作为可选格式（`--format json`），仅在需要管道/脚本处理时使用
- 提供 MCP Server 模式（stdio），供 AI 工具直接调用
- 提供通用 AI Skills 配置文件（不绑定特定工具）

---

## 三、CLI 命令设计

### 论文管理

| 命令 | 说明 |
|------|------|
| `paper add` | 添加论文（手动指定字段、DOI、arXiv ID、BibTeX 文件） |
| `paper get <id>` | 查看论文详情（支持 compact/verbose/json 格式） |
| `paper update <id>` | 更新论文信息（修改标题、链接、已读状态等） |
| `paper delete <id>` | 删除论文 |

### 列表

| 命令 | 说明 |
|------|------|
| `paper list` | 分页列出论文（支持 --page、--page-size、--tag、--group、--status、--sort、--format） |
| `paper count` | 返回匹配论文数量（支持 --tag、--group、--status 筛选） |

### 标签

| 命令 | 说明 |
|------|------|
| `paper tag add <id>` | 给论文添加标签 |
| `paper tag remove <id>` | 移除论文标签 |
| `paper tag list` | 列出所有标签（支持树形展示） |

### 分组

| 命令 | 说明 |
|------|------|
| `paper group add <name>` | 创建分组 |
| `paper group delete <name>` | 删除分组 |
| `paper group list` | 列出所有分组及其论文数量 |
| `paper group assign <id>` | 将论文加入分组 |
| `paper group unassign <id>` | 将论文移出分组 |

### 笔记

| 命令 | 说明 |
|------|------|
| `paper note add <id>` | 添加笔记（指定类型和内容） |
| `paper note list <id>` | 列出某论文的所有笔记 |
| `paper note update <note-id>` | 更新笔记内容 |
| `paper note delete <note-id>` | 删除笔记 |
| `paper note search <keyword>` | 按关键词搜索笔记（分页） |

### 引用关系

| 命令 | 说明 |
|------|------|
| `paper cite add <from> <to>` | 添加引用关系（指定类型和强度） |
| `paper cite remove <from> <to>` | 移除引用关系 |
| `paper cite graph <id>` | 查询关系子图（指定深度，支持 mermaid 格式输出） |

### 搜索

| 命令 | 说明 |
|------|------|
| `paper search <query>` | 全文搜索（支持 --in、--tag、--group、--fuzzy、--page） |

### 导出

| 命令 | 说明 |
|------|------|
| `paper export` | 导出数据（支持 bibtex、markdown、mermaid、json 格式） |

### 统计

| 命令 | 说明 |
|------|------|
| `paper stats` | 显示统计概览（支持 --tag-cloud） |

### AI 集成

| 命令 | 说明 |
|------|------|
| `paper serve` | 启动 HTTP Server（同时提供 Web 管理后台和 API） |
| `paper mcp` | 启动 MCP Server（stdio 模式，供 AI 工具调用） |

### 安装

| 命令 | 说明 |
|------|------|
| `ref setup` | 首次安装：复制二进制到 ~/.refstore/bin/，自动检测 shell 配置文件（.zshrc/.bashrc/.bash_profile/.profile）并注册 PATH |

---

## 四、Web 管理后台

### 定位

轻量级数据库管理后台，供人类查看和简单操作文献数据。类似 adminer 的定位，不做复杂交互。

### 功能

- 论文列表页（分页表格，显示标题、作者、标签、状态、日期）
- 论文详情页（查看完整信息、笔记、引用关系）
- 删除论文
- 标签管理（查看、删除）
- 分组管理（查看、删除、论文归类）
- 简单搜索

### 技术方案

- 服务端渲染（不搞前后端分离，减少复杂度）
- 与 API Server 共用一个二进制，通过 `paper serve` 启动
- 图关系可视化用轻量 JS 库（vis-network 或 sigma.js）内嵌到页面

---

## 五、技术栈

| 层面 | 选型 | 理由 |
|------|------|------|
| 语言 | Rust (edition 2021+) | 性能、单二进制部署、类型安全 |
| CLI 框架 | clap v4 (derive) | Rust 生态标准，声明式定义命令 |
| 数据库 | SQLite (rusqlite) | 零依赖，单文件，备份即拷贝 |
| 全文搜索 | SQLite FTS5 | 先用内置方案，够用且无需额外依赖；后续可加 tantivy |
| 图关系 | SQLite 邻接表 + 递归 CTE | 论文引用图规模小（通常 <10k 节点），完全胜任 |
| HTTP 后端 | axum (tokio) | Rust 生态最活跃的 web 框架 |
| 模板引擎 | askama | 编译时检查，类型安全，服务端渲染 HTML |
| 图可视化 | vis-network 或 sigma.js | 轻量，嵌入页面即可展示关系图 |
| MCP Server | JSON-RPC over stdio | 手写协议层，更可控 |
| 序列化 | serde + serde_json | Rust 标配 |
| HTTP 客户端 | reqwest | 抓取 DOI/arXiv 元数据 |

---

## 六、项目结构

```
papermanager/
├── Cargo.toml                    # workspace root
├── crates/
│   ├── core/                     # 核心库：数据模型、存储、搜索、导入导出
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── model.rs          # 数据模型
│   │       ├── db.rs             # SQLite 存储层
│   │       ├── search.rs         # 全文搜索
│   │       ├── graph.rs          # 引用关系图查询
│   │       ├── import.rs         # DOI/arXiv/BibTeX 导入
│   │       └── export.rs         # BibTeX/Markdown/Mermaid 导出
│   ├── cli/                      # CLI 入口
│   │   └── src/
│   │       ├── main.rs
│   │       ├── commands/         # 各子命令实现
│   │       └── output.rs         # 格式化输出（table/json/compact）
│   ├── server/                   # HTTP Server + Web 管理后台
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/           # API 路由
│   │       ├── templates/        # askama HTML 模板
│   │       └── middleware.rs
│   └── mcp/                      # MCP Server (stdio)
│       └── src/
│           └── main.rs
├── web/                          # 静态资源（JS/CSS，图可视化库）
├── skills/                       # AI Skills（通用，不绑定特定工具）
│   ├── paper-add.md
│   ├── paper-search.md
│   ├── paper-graph.md
│   └── paper-notes.md
├── .claude/
│   └── skills/                   # Claude Code 符号链接 → ../../skills/
├── DESIGN.md                     # 本文档
└── README.md
```

---

## 七、AI Skills 规划

Skills 是通用的 Markdown 指令文档，描述"遇到某类用户需求时执行什么 CLI 命令"。不绑定任何特定 AI 编码工具。

各工具的集成方式：
- **Claude Code**：通过 `.claude/skills/` 符号链接指向 `skills/` 目录
- **Codex**：将 skills 内容写入项目指令文件或 AGENTS.md
- **OpenCode**：将 skills 内容写入项目配置
- 其他工具：参照各自的指令/agent 配置方式引入

| Skill | 职责 |
|-------|------|
| paper-add | 指导 AI 通过 DOI/arXiv/手动方式添加论文 |
| paper-search | 指导 AI 构造搜索命令（关键词、标签、分组、笔记），理解返回结果 |
| paper-graph | 指导 AI 查询和展示论文引用关系图 |
| paper-notes | 指导 AI 添加、搜索和管理笔记 |

Skills 的核心原则：

- 默认使用格式化文本输出，AI 和人类都能直接阅读，无需额外解析
- JSON 格式（`--format json`）仅在需要管道传递或脚本处理时使用
- 列表操作始终带 `--page-size 10`，防止上下文溢出
- 先用 `count` 命令了解数据量，再决定是否翻页
- Skills 内容只描述命令用法和业务逻辑，不包含任何特定工具的配置语法

---

## 八、开发路线图

### Phase 1：核心数据层

- 初始化 Cargo workspace 项目结构
- 定义数据模型
- SQLite schema 设计与实现
- 基础 CRUD 操作（增删改查）
- CLI 命令：add / get / list / delete（含分页）

### Phase 2：搜索与标签

- SQLite FTS5 全文搜索实现
- 标签 CRUD（含层级标签）
- 笔记 CRUD
- CLI 命令：search / tag / note

### Phase 3：引用关系与导入导出

- 引用关系 CRUD
- 图查询（递归 CTE）
- DOI / arXiv 元数据自动抓取
- BibTeX 导入 / 导出
- Markdown / Mermaid 导出
- CLI 命令：cite / import / export

### Phase 4：Web 管理后台

- axum HTTP Server
- REST API（复用 core 层逻辑）
- askama 模板渲染管理页面
- 论文列表、详情、删除功能
- 标签管理页
- 图关系可视化（嵌入 vis-network）

### Phase 5：AI 集成

- MCP Server（stdio 模式）
- 通用 AI Skills 编写
- 统计功能（stats 命令）
- JSON 完整备份与恢复
- 文档与 README
