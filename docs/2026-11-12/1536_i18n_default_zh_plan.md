# 界面中文本地化（默认 zh-CN）— 方案与实施记录

- 时间：2026-11-12 15:36
- 分支：`feat/i18n-zh-default`
- 范围确认：用户选择 A（CLI 与错误文案）+ B（TUI 界面文案）；接受多语言并存，当前目标是界面默认中文。
- 不在范围内：Rust 源码注释、`tests/`、`docs/evidence|contracts`、`CHANGELOG.md`、模型可见提示词（工具描述、系统提示）。

---

## 1. 需求

1. 界面文案默认中文（`zh-CN`），英文保留为可切换语言。
2. 覆盖范围：`pi --help` / 子命令 help、错误提示（summary + hints）、TUI 标签与状态、斜杠命令说明、快捷键说明、设置页。
3. 不改变既有英文行为：以 `locale=en` 运行时输出必须与改动前逐字节一致，以此保护既有快照与断言。

---

## 2. 现状摸底（2026-11-12 实测）

| 区域 | 文件 | 规模 | 可翻译文案（估算） |
| --- | --- | --- | --- |
| CLI 结构 | `src/cli.rs` | 3,635 行 | 378 条 `///` help 行、207 个 `#[arg]` |
| CLI 输出 | `src/main.rs` | 18,972 行 | 326 条英文散文式字面量、360 个输出点 |
| 错误提示（静态表） | `src/error_hints.rs` | 1,439 行 | 157 条 |
| 错误提示（结构体） | `src/error.rs` | 1,700+ 行（`#[cfg(test)]` 前） | 226 条 |
| 应用层提示 | `src/app.rs` | — | 44 条 |
| 斜杠命令说明 | `src/autocomplete.rs` | 2,851 行 | 50 条 |
| TUI（默认栈） | `src/interactive_ftui.rs` 及子模块 | 34,565 行 | 约 400 条（含短标签） |
| TUI（经典栈） | `src/interactive/*.rs` | 约 15 个文件 | 约 300 条 |
| 状态栏 | `src/status_line.rs` | 487 行 | 约 10 条 |

约束性事实：

- 项目**没有任何 i18n 基础设施**：无 gettext / fluent / rust_i18n，无 `locale/` 资源目录，无 `--lang`，无 `language` 设置项。
- 文案以字面量形式内联在渲染逻辑与 `format!` 中，不是集中资源表。
- 测试侧锁定面很大：`tests/snapshots/` 33 个快照、`tests/*.rs` 中 5,057 行断言引用英文字符串。
- `src/error_hints.rs` 与 `src/error.rs::hints()` 是**两套并行**的错误提示系统，都必须本地化。

结论：全量改写为 key + 资源表会触碰 700+ 调用点并使 5,000+ 条英文断言失效，风险与收益不成比例。采用“以英文原文为 key、在渲染边界翻译”的方案（见下）。

---

## 3. 设计

### 3.1 目录与查表

新增 `src/i18n.rs`：

- `Locale`：`En`、`ZhCn`；解析 `zh`、`zh-CN`、`zh_CN`、`en`、`en-US` 等标签。
- `tr(en_text: &'static str) -> &'static str`：按当前 locale 查表；未收录的 key 回退英文。
- `t_format(template: &'static str, args: &[(&str, &str)]) -> String`：模板中的 `{name}` 占位符替换，用于带插值的文案。
- 翻译表为 `const` 数组 `&[(&str, &str)]`，以英文原文排序后二分查找。
- 单测保证：表内 key 唯一、zh 非空、`tr` 在 `En` 下恒等。

以英文原文为 key 的理由：

1. `locale=en` 时天然逐字节不变，既有测试与快照零改动。
2. 可增量翻译，未翻译条目自动回退英文，不产生空字符串。
3. 无需改动 700 个调用点的类型与生命周期。

### 3.2 语言选择优先级

`--lang <tag>` > `PI_LANG` > `settings.json` 的 `language` > `LANG` / `LC_ALL` > 默认 `zh-CN`。

进程级 `OnceLock<Locale>` 保存解析结果；纯函数式接口（`tr_in(locale, ..)`）供测试显式指定。

### 3.3 翻译边界

翻译在**输出边界**进行，不在构造点进行：

- 错误提示：`ErrorHint` / `ErrorHints` 保持英文构造，新增 `localized(locale)` 在渲染前映射。
- CLI help：保留 clap derive 的英文作为回退，在 `Cli::command()` 构建后按 arg id 覆盖 `about` / `help` / `long_about`（clap 官方支持的运行时改写路径），避免重写 378 行属性。
- TUI：在拼接状态行、帮助文本、命令说明、提示语时调用 `tr`。

### 3.4 验证门槛

- `locale=en` 下：`--help` 输出、错误提示、快照全部不变。
- `locale=zh-CN` 下：抽样断言中文文案出现且不含英文回退（对已翻译条目）。
- 契约性标识符（子命令名、flag 名、错误码、`provider/id`、键名）**不翻译**。

---

## 4. 实施阶段

| 阶段 | 内容 | 独立性 |
| --- | --- | --- |
| P1 | `src/i18n.rs` + 语言解析 + `--lang` / `PI_LANG` / settings 接线 | 独立可验证 |
| P2 | 错误提示本地化（`error_hints.rs` + `error.rs`） | 依赖 P1 |
| P3 | CLI help 本地化（顶层选项 + 高频子命令） | 依赖 P1 |
| P4 | TUI 共享面：状态栏、帮助层、斜杠命令说明、快捷键页 | 依赖 P1 |
| P5 | TUI 其余面板（设置页、树/分叉、工作区报告） | 依赖 P4 |

每阶段单独提交，失败可整段回滚。

---

## 5. 环境约束与验证口径（必须如实记录）

本次执行环境与仓库要求不一致，**无法满足 AGENTS.md 规定的质量门槛**：

- 仓库 `rust-toolchain.toml` 固定 `nightly-2026-08-31`，本机 `rustup` 报告 `no release found`，该工具链无法安装。
- 仓库要求的 `dsr quality --tool pi_agent_rust` 与 RCH 在本机不存在。
- 本机可用工具链为 `nightly 1.101.0 (2026-10-09)` 与 `stable 1.97.1`；构建只能以 `cargo +nightly` 覆盖固定版本进行。
- 仓库无 `target/` 目录，首次构建为全量冷编译。

因此本地只能给出 `cargo +nightly check` / 定向测试的**静态验证**结论，不构成质量或发布声明；合入前仍需在具备固定工具链与 DSR 的环境中跑完整门禁。

---

## 6. 风险

1. 快照与断言面大：TUI 文案改动必然触碰 `tests/snapshots/` 与 `interactive_ftui` 内部断言，需要在 `locale=en` 路径上保持恒等来规避。
2. clap 运行时覆盖依赖 arg id 稳定性；字段重命名会静默失去翻译（表现为回退英文），需用测试固定已翻译 id 集合。
3. 中文宽度与换行：TUI 使用 `display_width`，中文为双宽字符，状态行与帮助层的截断逻辑需要按显示宽度复核。
4. 译文一致性：术语（会话 / 分支 / 思考等级 / 工具）需统一，避免同一概念多种中文写法。

---

## 7. 实施记录（2026-11-12 16:20）

### P1 本地化骨架

- 新增 `src/i18n.rs`：`Locale`（`En` / `ZhCn`）、标签解析、优先级解析、进程级当前语言（`AtomicU8`，未初始化时为英文）、`tr` / `tr_in` / `t_format` / `t_format_in`、clap 命令树本地化。
- 消息目录按域拆分：`ZH_CATALOG`（通用）、`ZH_UI`（斜杠命令与界面提示）、`ZH_HINTS`（错误提示）、`ZH_CLI_ARGS` / `ZH_CLI_SUBCOMMANDS`（命令行帮助）。查表为 `LazyLock<HashMap>`，未收录回退英文原文。
- 新增 CLI 选项 `--lang <TAG>`；参与 `known_long_option` 预扫描，避免被当作扩展参数。
- 新增设置项 `Config.language`（别名 `locale`），并纳入 `Config::merge`。

语言优先级：`--lang` > `PI_LANG` > `settings.json.language` > `zh-CN`。

不读取 `LANG` / `LC_ALL`：容器与 CI 里常见的 `en_US.UTF-8` 不应把中文界面改成英文；需要英文时用 `--lang en` 或 `PI_LANG=en`。

### P2 错误提示

- `error_hints::format_error_with_hints` 改为按当前语言渲染，新增显式版本 `format_error_with_hints_in(locale, error)`。
- `Error:`、`Suggestions:` 标签与全部 `summary` / hint 文案纳入 `ZH_HINTS`；正文（`Error` 的 `Display`）保持英文，作为技术细节。

### P3 命令行帮助

- `cli::parse_with_extension_flags` 先按 `--lang` / 环境解析语言，再用 `localize_command` 构建命令树，经 `try_get_matches_from` + `from_arg_matches` 解析。
- 已翻译：顶层 `about`、`after_help`、约 77 个顶层选项帮助、30 个子命令简介。
- 未翻译：子命令内部选项帮助（回退英文）。

### P4 TUI 共享面

- 状态行：`ready` / `working` / `thinking ...` / `responding ...` / `running {tool} ...` / `todo {todo}`。
- 输入框占位符、选择器底部提示、未知命令提示、`/help` 头部。
- 斜杠命令说明（`autocomplete.rs` 全部 50 条）。
- 经典 TUI 的 `/help` 全文（新增 `help_text_for(locale)` 与 `ZH_HELP_TEXT`）。

### 顺带修复的编译阻断（与本地化无关，HEAD 本身无法编译）

1. `src/web_remote/companion.rs`：`crate::Result` 不存在，改为已导出的 `crate::PiResult`。
2. `src/extensions.rs`：`rquickjs` 的 `IntoArgs` 只实现到 7 元组，而 `__pi_execute_tool` 需要 9 个实参（桥接密钥 + 8 个业务参数）。新增 `call_js_spread`，用 `Reflect.apply` 展开参数数组，JS 侧签名与探测脚本均不变。

两项均未改变对外行为，但属于本任务范围之外的必要修复，单独说明。

---

## 8. 验证结果（如实记录）

| 项 | 结果 |
| --- | --- |
| `cargo +nightly check --lib --keep-going` | 通过（0 error；2 warning 与本改动无关） |
| `cargo +nightly check --bin pi --keep-going` | 通过 |
| `cargo +nightly test --lib` | **无法编译**：`rustix::fs::mkfifoat` 在 macOS 上不存在（`src/extensions_js_storage.rs`、`src/subagents/task_file/tests.rs`、`src/url_router/skills.rs`），属既有问题 |
| `dsr quality --tool pi_agent_rust` | 未执行：本机无 `dsr` |
| 固定工具链 `nightly-2026-08-31` | 不可用（`no release found`），验证使用本机 `nightly 1.101.0` |

因此本次只能声明“静态编译通过”，不能声明质量门禁通过或测试通过。

---

## 9. 遗留项

1. 子命令内部选项帮助（约 226 行文档注释）尚未翻译。
2. `src/error.rs::Error::hints()` 是第二套提示系统（226 条文案），尚未接入本地化。
3. TUI 其余面板：设置页、工具进度、树/分叉覆盖层、工作区报告、快捷键目录（`keybindings.rs` 约 150 条动作说明）。
4. 经典 TUI（`--classic`）除 `/help` 外的文案。
5. 中文双宽字符对状态行截断的影响需在真实终端复核。
