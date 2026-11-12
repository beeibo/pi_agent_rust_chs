//! 界面本地化：语言解析、消息目录与查表。
//!
//! 设计要点：
//! - 英文原文即查表键。未收录的条目回退英文，因此英文运行路径与本地化前逐字节一致。
//! - 语言优先级：`--lang` > `PI_LANG` > 设置项 `language` > `zh-CN`。
//!   不读取 `LANG` / `LC_ALL`，避免容器里常见的 `en_US.UTF-8` 把界面改成英文。
//! - [`current`] 在未显式初始化时返回 [`Locale::En`]，让库级测试与快照保持英文；
//!   CLI 启动时由 [`init`] 按上面的优先级解析，未配置时落到中文。
//! - 翻译发生在输出边界，构造期仍保留英文，避免改动大量调用点与既有断言。

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// 界面语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Locale {
    /// 英语（回退语言）。
    #[default]
    En,
    /// 简体中文。
    ZhCn,
}

impl Locale {
    /// 解析 BCP-47 风格的语言标签（大小写、`_` / `-` 均可）。
    #[must_use]
    pub fn parse(tag: &str) -> Option<Self> {
        let tag = tag.trim().replace('_', "-").to_ascii_lowercase();
        if tag.is_empty() {
            return None;
        }
        let primary = tag.split('-').next().unwrap_or_default();
        match primary {
            "zh" => Some(Self::ZhCn),
            "en" => Some(Self::En),
            _ => None,
        }
    }

    /// 规范标签。
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::ZhCn => "zh-CN",
        }
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.tag())
    }
}

/// 进程级已解析语言。`0` 表示尚未初始化，此时 [`current`] 返回 [`Locale::En`]。
static ACTIVE: AtomicU8 = AtomicU8::new(0);

const ACTIVE_UNSET: u8 = 0;
const ACTIVE_EN: u8 = 1;
const ACTIVE_ZH_CN: u8 = 2;

const fn encode(locale: Locale) -> u8 {
    match locale {
        Locale::En => ACTIVE_EN,
        Locale::ZhCn => ACTIVE_ZH_CN,
    }
}

/// 默认语言：没有任何配置时使用中文。
pub const DEFAULT_LOCALE: Locale = Locale::ZhCn;

/// 当前生效语言。
#[must_use]
pub fn current() -> Locale {
    match ACTIVE.load(Ordering::Relaxed) {
        ACTIVE_ZH_CN => Locale::ZhCn,
        _ => Locale::En,
    }
}

/// 显式设置当前语言。
///
/// 启动早期先按 `--lang` / 环境变量解析一次，配置加载完成后可用设置项再覆盖，
/// 因此允许重复设置。
pub fn set(locale: Locale) {
    ACTIVE.store(encode(locale), Ordering::Relaxed);
}

/// 按优先级解析语言。
///
/// `explicit` 来自 `--lang`，`setting` 来自设置文件。只有显式来源才能选择英文；
/// 其余情况一律回落到 [`DEFAULT_LOCALE`]（中文）。不读取 `LANG` / `LC_ALL`：
/// 容错环境里常见的 `LANG=en_US.UTF-8` 不应把界面改成英文。
#[must_use]
pub fn resolve(explicit: Option<&str>, setting: Option<&str>) -> Locale {
    explicit
        .and_then(Locale::parse)
        .or_else(|| {
            std::env::var("PI_LANG")
                .ok()
                .as_deref()
                .and_then(Locale::parse)
        })
        .or_else(|| setting.and_then(Locale::parse))
        .unwrap_or(DEFAULT_LOCALE)
}

/// 解析并安装当前语言，返回最终取值。
pub fn init(explicit: Option<&str>, setting: Option<&str>) -> Locale {
    let locale = resolve(explicit, setting);
    set(locale);
    locale
}

/// 中文消息目录：英文原文 → 简体中文。
///
/// 条目按英文原文升序排列，查表使用二分查找；新增条目必须保持有序，
/// `catalog_is_sorted_and_unique` 测试会校验。
#[rustfmt::skip]
const ZH_CATALOG: &[(&str, &str)] = &[
    ("(empty)", "(空)"),
    ("A newer Pi release is available", "有可用的新版本 Pi"),
    ("API key not configured", "未配置 API key"),
    ("Aborted", "已中止"),
    ("Agent", "智能体"),
    ("Apply", "应用"),
    ("Cancel", "取消"),
    ("Close", "关闭"),
    ("Configuration error", "配置错误"),
    ("Continue", "继续"),
    ("Copy", "复制"),
    ("Delete", "删除"),
    ("Deny", "拒绝"),
    ("Edit", "编辑"),
    ("Error", "错误"),
    ("Error:", "错误："),
    ("Suggestions:", "建议："),
    ("Failed", "失败"),
    ("Help", "帮助"),
    ("Loading...", "加载中..."),
    ("Model", "模型"),
    ("Next", "下一个"),
    ("No", "否"),
    ("OK", "确定"),
    ("Previous", "上一个"),
    ("Quit", "退出"),
    ("Retry", "重试"),
    ("Save", "保存"),
    ("Session", "会话"),
    ("Settings", "设置"),
    ("Success", "成功"),
    ("Thinking", "思考中"),
    ("Tools", "工具"),
    ("Warning", "警告"),
    ("Yes", "是"),
    ("ready", "就绪"),
    ("responding ...", "正在回复..."),
    ("running {tool} ...", "正在运行 {tool} ..."),
    ("thinking ...", "正在思考..."),
    ("todo {todo}", "待办 {todo}"),
    ("working", "处理中"),
];

/// 斜杠命令与设置项说明等长文本目录，与 [`ZH_CATALOG`] 同构但单独维护，
/// 便于按界面分区审查译文。
#[rustfmt::skip]
const ZH_UI: &[(&str, &str)] = &[
    ("Show help for interactive commands", "显示交互命令帮助"),
    ("Show keyboard shortcuts", "显示键盘快捷键"),
    ("Show changelog entries", "显示更新日志"),
    ("Show or change the current model", "查看或切换当前模型"),
    ("Show or change the model's serving tier", "查看或切换模型服务档位"),
    ("Show or change thinking level", "查看或设置思考等级"),
    ("Set thinking level (off/minimal/low/medium/high/xhigh/max)", "设置思考等级（off/minimal/low/medium/high/xhigh/max）"),
    ("Show input history", "显示输入历史"),
    ("Show session info", "显示会话信息"),
    ("Show session branch tree summary", "显示会话分支树概要"),
    ("Start a new session", "开始新会话"),
    ("Clear conversation history", "清空对话历史"),
    ("Exit Pi", "退出 Pi"),
    ("Pick and resume a previous session", "选择并恢复历史会话"),
    ("List or switch themes", "列出或切换主题"),
    ("Edit and save session preferences", "编辑并保存会话偏好"),
    ("Export conversation to HTML", "将会话导出为 HTML"),
    ("Copy last assistant message to clipboard", "复制上一条助手消息到剪贴板"),
    ("Set session display name", "设置会话显示名称"),
    ("Branch from a previous user message", "从上一条用户消息分叉"),
    ("Remove stored OAuth credentials", "删除已保存的 OAuth 凭据"),
    ("Open browser companion pairing or revoke browser access", "打开浏览器配对或撤销浏览器访问"),
    ("Show or set model scope patterns", "查看或设置模型范围匹配模式"),
    ("OAuth login (provider-specific)", "OAuth 登录（按提供方）"),
    ("Compact older context", "压缩较早的上下文"),
    ("Reload resources from disk", "从磁盘重新加载资源"),
    ("Share the session as a GitHub Gist", "将会话以 GitHub Gist 分享"),
    ("Show MCP server status (Model Context Protocol)", "查看 MCP 服务器状态（Model Context Protocol）"),
    ("Expand a prompt template by name", "按名称展开提示模板"),
    ("Enter or manage read-only plan mode", "进入或管理只读计划模式"),
    ("Manage goals (FrankenTUI only; /goal status)", "管理目标（仅 FrankenTUI；/goal status）"),
    ("Hand off to @smol after planning and the first edit", "规划并完成首次编辑后交给 @smol"),
    ("Second-model turn review (advisor)", "第二模型轮次复核（advisor）"),
    ("Mark a session restore point", "标记会话恢复点"),
    ("Collapse back to a checkpoint with a report", "回退到某个检查点并生成报告"),
    ("Reset provider cache and stream bookkeeping", "重置提供方缓存与流式状态"),
    ("Re-queue the last user turn", "重新排队上一条用户消息"),
    ("Revert the last recorded file mutation", "撤销最近一次记录的文件修改"),
    ("Replay an undone file mutation", "重放已撤销的文件修改"),
    ("Show provider credit/quota usage", "查看提供方额度与用量"),
    ("Show or change the tool approval mode", "查看或切换工具审批模式"),
    ("Emit a handoff document for the next agent", "为下一个智能体生成交接文档"),
    ("Show or manage stream rules", "查看或管理流规则"),
    ("Ask an ephemeral side question on the smol role", "用 smol 角色问一个不落盘的旁路问题"),
    ("Run tangential work in a background task-role child", "在后台任务子进程中运行旁支工作"),
    ("File a grievance and inject a stream rule", "记录一条不满并注入流规则"),
    ("Dependency-ordered atomic commit splitting", "按依赖顺序拆分原子提交"),
    ("Review the current changes", "复核当前改动"),
    ("Add a directory to the workspace roots", "将目录加入 workspace 根目录"),
    ("Remove a directory from the workspace roots", "从 workspace 根目录移除目录"),
    ("List recorded crash bundles", "列出已记录的崩溃包"),
    ("Browser message · Enter sends · host input retakes control", "浏览器消息 · 回车发送 · 宿主输入重新接管"),
    ("Type a message (Enter to send, Alt+Enter for newline)", "输入消息（回车发送，Alt+Enter 换行）"),
    ("type to filter · ↑/↓ navigate · Enter apply · Esc close", "输入以筛选 · ↑/↓ 导航 · Enter 应用 · Esc 关闭"),
    ("Unknown command: /{name} (try /help)", "未知命令：/{name}（可试 /help）"),
    ("Unknown command: /{name} (extensions disabled; try /help)", "未知命令：/{name}（扩展已禁用，可试 /help）"),
    ("/{name} is a pi command that this stack does not implement yet; run `pi --classic` for it", "/{name} 是 pi 命令，但当前界面尚未实现；可用 `pi --classic` 切换到经典界面"),
    ("pi commands:", "Pi 命令："),
];

/// 错误提示目录：`summary` 与 remediation 建议文案。
#[rustfmt::skip]
const ZH_HINTS: &[(&str, &str)] = &[
    ("API error", "API 错误"),
    ("API key is invalid or expired", "API key 无效或已过期"),
    ("API resource not found", "API 资源不存在"),
    ("Auth file locked by another process", "auth.json 被其他进程锁定"),
    ("Authentication error", "认证错误"),
    ("Check API documentation", "请查阅对应 API 文档"),
    ("Check API key permissions at your provider's console", "请在提供方控制台确认 API key 的权限"),
    ("Check API key permissions for this resource", "请确认 API key 对该资源的权限"),
    ("Check directory permissions", "请检查目录权限"),
    ("Check extension configuration", "请检查扩展配置"),
    ("Check extension manifest.json syntax", "请检查扩展 manifest.json 的语法"),
    ("Check extension name is correct", "请确认扩展名称正确"),
    ("Check file permissions", "请检查文件权限"),
    ("Check file system and permissions", "请检查文件系统与权限"),
    ("Check file/directory permissions", "请检查文件或目录权限"),
    ("Check input parameters", "请检查输入参数"),
    ("Check parameter types match expected schema", "请确认参数类型符合预期结构"),
    ("Check provider documentation for this error", "请查阅提供方文档中对该错误的说明"),
    ("Check provider status page for outages", "请查看提供方状态页是否在故障中"),
    ("Check that JSON fields match expected schema", "请确认 JSON 字段符合预期结构"),
    ("Check that the model ID is correct", "请确认模型 ID 正确"),
    ("Check that ~/.pi/agent/settings.json exists and is valid JSON", "请确认 ~/.pi/agent/settings.json 存在且为合法 JSON"),
    ("Check that 'providers' key exists in models.json", "请确认 models.json 中存在 'providers' 键"),
    ("Check your API credentials", "请检查 API 凭据"),
    ("Check your internet connection", "请检查网络连接"),
    ("Close other Pi instances that may be using auth.json", "请关闭其他可能在使用 auth.json 的 Pi 实例"),
    ("Close other Pi instances using this database", "请关闭其他使用该数据库的 Pi 实例"),
    ("Close other Pi instances using this session", "请关闭其他使用该会话的 Pi 实例"),
    ("Command failed with non-zero exit code", "命令以非零退出码结束"),
    ("Command timed out", "命令超时"),
    ("Consider breaking into smaller commands", "可拆分成更小的命令"),
    ("Consider using a different model or reducing request frequency", "可改用其他模型或降低请求频率"),
    ("Consider using a smaller context or simpler request", "可减小上下文或简化请求"),
    ("Database corrupted", "数据库已损坏"),
    ("Database error", "数据库错误"),
    ("Database locked", "数据库被锁定"),
    ("Delete ~/.pi/agent/sessions/index.db to rebuild", "删除 ~/.pi/agent/sessions/index.db 以重建索引"),
    ("Extension capability denied", "扩展能力被拒绝"),
    ("Extension error", "扩展错误"),
    ("Extension not found", "扩展不存在"),
    ("Extension requires capabilities not granted by policy", "扩展需要当前策略未授予的能力"),
    ("File already exists", "文件已存在"),
    ("File not found", "文件不存在"),
    ("File or directory not found", "文件或目录不存在"),
    ("Forbidden API request", "API 请求被禁止"),
    ("GitHub Copilot OAuth client_id not configured", "未配置 GitHub Copilot OAuth client_id"),
    ("I/O error", "I/O 错误"),
    ("If running tests, set VCR_MODE=record to create cassettes", "运行测试时设置 VCR_MODE=record 以生成 cassette"),
    ("If using a proxy, verify proxy settings", "若使用代理，请检查代理配置"),
    ("Increase timeout with 'timeout' parameter", "可通过 'timeout' 参数调大超时"),
    ("Install fd: 'apt install fd-find' or 'brew install fd'", "安装 fd：'apt install fd-find' 或 'brew install fd'"),
    ("Invalid JSON syntax", "JSON 语法错误"),
    ("Invalid extension manifest", "扩展 manifest 无效"),
    ("Invalid models configuration", "models 配置无效"),
    ("Invalid or missing configuration file", "配置文件无效或缺失"),
    ("Invalid parameter type", "参数类型无效"),
    ("Invalid regex pattern", "正则表达式无效"),
    ("JSON data does not match expected structure", "JSON 数据结构与预期不符"),
    ("JSON error", "JSON 错误"),
    ("Local providers (Ollama/LM Studio): the first request can block while the model loads — ensure the model is pulled (ollama pull <model>) and the server is reachable (ollama list)", "本地提供方（Ollama/LM Studio）：首次请求可能因模型加载而阻塞——请确认模型已拉取（ollama pull <model>）且服务可访问（ollama list）"),
    ("Model not found or unavailable", "模型不存在或不可用"),
    ("Multiple matches found for replacement", "替换目标匹配到多处"),
    ("Network connection error", "网络连接错误"),
    ("OAuth token expired or invalid", "OAuth 令牌已过期或无效"),
    ("Operation cancelled by user", "操作已被用户取消"),
    ("Or add key to ~/.pi/agent/auth.json", "或将 key 写入 ~/.pi/agent/auth.json"),
    ("Or ensure VCR_CASSETTE_DIR contains the expected cassette file", "或确认 VCR_CASSETTE_DIR 包含预期的 cassette 文件"),
    ("Or run on a workstation with a browser, or use device flow over SSH (set PI_COPILOT_FORCE_DEVICE_FLOW=1)", "或在带浏览器的机器上运行，或通过 SSH 使用设备流（设置 PI_COPILOT_FORCE_DEVICE_FLOW=1）"),
    ("Or set API key directly via environment variable", "或直接通过环境变量设置 API key"),
    ("Permission denied", "权限不足"),
    ("Permission denied reading file", "读取文件权限不足"),
    ("Permission denied writing file", "写入文件权限不足"),
    ("Provider API error", "提供方 API 错误"),
    ("Provider server error", "提供方服务端错误"),
    ("Provide all required parameters", "请提供全部必需参数"),
    ("Provide more context in old_text to make it unique", "请在 old_text 中提供更多上下文以保证唯一匹配"),
    ("Rate limit exceeded", "已触发速率限制"),
    ("Raise --max-cost / session.maxCost explicitly to continue this session", "如需继续该会话，请显式调高 --max-cost / session.maxCost"),
    ("Raise the timeout: --request-timeout <seconds>, PI_HTTP_REQUEST_TIMEOUT_SECS=<seconds>, or requestTimeoutSecs in settings.json (0 = no timeout)", "调大超时：--request-timeout <秒>、PI_HTTP_REQUEST_TIMEOUT_SECS=<秒>，或 settings.json 中的 requestTimeoutSecs（0 表示不超时）"),
    ("Request timed out", "请求超时"),
    ("Required field missing", "缺少必需字段"),
    ("Retry the request; if it persists, temporarily disable VPN/antivirus/firewall software to identify the interfering layer", "重试请求；若持续失败，可临时关闭 VPN / 杀毒 / 防火墙以定位干扰层"),
    ("Review command output for error details", "请查看命令输出以获取错误详情"),
    ("Review extension security settings", "请复核扩展的安全设置"),
    ("Review the tool parameters and try again", "请复核工具参数后重试"),
    ("Run 'pi --resume' to pick from existing sessions", "运行 'pi --resume' 从已有会话中选择"),
    ("Run 'pi config' to see configuration paths and precedence", "运行 'pi config' 查看配置路径与优先级"),
    ("Run 'pi login <provider>' to re-authenticate", "运行 'pi login <provider>' 重新认证"),
    ("Session error", "会话错误"),
    ("Session file is corrupted or invalid", "会话文件已损坏或无效"),
    ("Session file is locked by another process", "会话文件被其他进程锁定"),
    ("Session file not found", "会话文件不存在"),
    ("Session files are JSONL format - check for malformed lines", "会话文件为 JSONL 格式——请检查是否有格式错误的行"),
    ("Session model-spend cap reached", "会话模型花费已达上限"),
    ("Set ANTHROPIC_API_KEY environment variable", "设置 ANTHROPIC_API_KEY 环境变量"),
    ("Set GITHUB_COPILOT_CLIENT_ID to your GitHub OAuth App / GitHub App client id", "将 GITHUB_COPILOT_CLIENT_ID 设为你的 GitHub OAuth App / GitHub App client id"),
    ("Socket not connected (Windows WSAENOTCONN 10057) - often VPN, antivirus, or Winsock LSP interference", "套接字未连接（Windows WSAENOTCONN 10057）——通常是 VPN、杀毒软件或 Winsock LSP 干扰"),
    ("Start a new session with 'pi'", "使用 'pi' 开始新会话"),
    ("Text to replace not found in file", "待替换的文本未在文件中找到"),
    ("The binary may be named 'fdfind' on some systems", "某些系统上该可执行文件名为 'fdfind'"),
    ("The session index may need to be rebuilt", "可能需要重建会话索引"),
    ("This is a temporary issue - try again shortly", "这是临时性问题，请稍后重试"),
    ("Tool execution error", "工具执行错误"),
    ("Try again - the provider may be slow", "请重试——提供方可能较慢"),
    ("Try starting a new session with 'pi'", "可尝试用 'pi' 开始新会话"),
    ("Unauthorized API request", "API 请求未获授权"),
    ("Use 'ls' or 'find' to locate the file", "使用 'ls' 或 'find' 定位该文件"),
    ("Use 'pi --list-models' to see available models", "使用 'pi --list-models' 查看可用模型"),
    ("Use 'pi list' to see installed extensions", "使用 'pi list' 查看已安装扩展"),
    ("Use 'pi' without --session to start a new session", "使用不带 --session 的 'pi' 开始新会话"),
    ("Use 'read' to see the current file content", "使用 'read' 查看当前文件内容"),
    ("Use a different path or remove existing file first", "请改用其他路径，或先删除已有文件"),
    ("VCR cassette missing or invalid", "VCR cassette 缺失或无效"),
    ("Validate JSON at jsonlint.com or similar", "可用 jsonlint.com 等工具校验 JSON"),
    ("Validation error", "校验错误"),
    ("Verify JSON syntax and structure", "请检查 JSON 的语法与结构"),
    ("Verify required fields are present", "请确认必需字段均已提供"),
    ("Verify the file path is correct", "请确认文件路径正确"),
    ("Verify the old_text exactly matches content in the file", "请确认 old_text 与文件内容完全一致"),
    ("Verify the path exists", "请确认路径存在"),
    ("Verify your API key is correct and active", "请确认 API key 正确且有效"),
    ("Verify ~/.pi/agent/models.json has valid JSON syntax", "请确认 ~/.pi/agent/models.json 的 JSON 语法合法"),
    ("Wait a moment and try again", "请稍候重试"),
    ("fd command not found", "未找到 fd 命令"),
    ("Check database file permissions and integrity", "请检查数据库文件权限与完整性"),
    ("Check for missing commas, brackets, or quotes", "请检查是否缺少逗号、括号或引号"),
    ("Check regex syntax - special characters may need escaping", "请检查正则语法——特殊字符可能需要转义"),
    ("Check the API endpoint URL", "请检查 API 端点 URL"),
    ("Check configuration file syntax and required fields", "请检查配置文件的语法与必需字段"),
    ("Inspect layered providers with 'netsh winsock show catalog'; as a last resort run 'netsh winsock reset' from an elevated prompt and reboot", "用 'netsh winsock show catalog' 检查分层提供方；万不得已时在提权提示符下运行 'netsh winsock reset' 并重启"),
    ("Use 'pi --resume' to pick from existing sessions", "使用 'pi --resume' 从已有会话中选择"),
    ("netsh winsock reset", "netsh winsock reset"),
];

/// 合并后的索引。各目录的键必须互不重复。
static INDEX: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::with_capacity(ZH_CATALOG.len() + ZH_UI.len() + ZH_HINTS.len());
    for (source, translated) in ZH_CATALOG.iter().chain(ZH_UI.iter()).chain(ZH_HINTS.iter()) {
        map.insert(*source, *translated);
    }
    map
});

/// 按当前语言翻译静态文案；未收录时回退英文原文。
#[must_use]
pub fn tr(source: &'static str) -> &'static str {
    tr_in(current(), source)
}

/// 按指定语言翻译静态文案；未收录时回退英文原文。
#[must_use]
pub fn tr_in(locale: Locale, source: &'static str) -> &'static str {
    match locale {
        Locale::En => source,
        Locale::ZhCn => INDEX.get(source).copied().unwrap_or(source),
    }
}

/// 翻译带 `{name}` 占位符的模板并填充参数。
#[must_use]
pub fn t_format(template: &'static str, args: &[(&str, &str)]) -> String {
    t_format_in(current(), template, args)
}

/// [`t_format`] 的显式语言版本。
#[must_use]
pub fn t_format_in(locale: Locale, template: &'static str, args: &[(&str, &str)]) -> String {
    let text = tr_in(locale, template);
    if args.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 16);
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let (head, tail) = rest.split_at(open);
        out.push_str(head);
        let after = &tail[1..];
        match after.find('}') {
            Some(close) => {
                let key = &after[..close];
                match args.iter().find(|(name, _)| *name == key) {
                    Some((_, value)) => out.push_str(value),
                    None => {
                        out.push('{');
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            None => {
                out.push_str(tail);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

// ─────────────────────────── CLI 帮助本地化 ───────────────────────────

/// 顶层选项 id → 中文帮助。
#[rustfmt::skip]
const ZH_CLI_ARGS: &[(&str, &str)] = &[
    ("add_dir", "额外的 workspace 根目录（可重复）：授予智能体访问主工作目录之外目录的权限"),
    ("advisor", "`advisor` 角色的模型：provider/model，可带 :thinking 后缀"),
    ("api_key", "API key（覆盖环境变量）"),
    ("append_system_prompt", "在系统提示后追加内容（文本或文件路径）"),
    ("approval_mode", "工具审批模式：always-ask（默认）、write 或 yolo"),
    ("classic", "改用经典 charmed_rust TUI，而非默认的 ftui"),
    ("continue", "继续上一个会话"),
    ("crash_test", "触发一次崩溃以验证崩溃包管线（隐藏诊断用）"),
    ("explain_extension_policy", "打印解析后的扩展策略与逐项能力决策后退出"),
    ("explain_repair_policy", "打印解析后的修复策略后退出"),
    ("extension", "加载扩展文件（可重复）"),
    ("extension_policy", "扩展能力策略：safe、balanced 或 permissive（兼容别名 standard）"),
    ("ftui", "启动 FrankenTUI 交互界面（构建启用 ftui 时的默认值）"),
    ("hide_cwd_in_prompt", "在系统提示中隐藏当前工作目录"),
    ("inline", "使用 ftui 时内联运行：界面在底部，保留 shell 滚动历史"),
    ("max_cost", "会话树累计花费达到该美元金额后拒绝新的模型请求"),
    ("max_time", "单次运行的墙钟上限（秒）"),
    ("max_tool_iterations", "每轮智能体允许的工具调用迭代上限，默认 50，范围 [1, 1000]"),
    ("model", "模型 ID（例如 claude-opus-4、gpt-4o）"),
    ("models", "Ctrl+P 循环切换的模型匹配模式（逗号分隔，支持通配符）"),
    ("mode", "输出模式（text、json、rpc）"),
    ("no_context_files", "禁用 AGENTS.md / CLAUDE.md 的发现与加载"),
    ("no_extensions", "禁用扩展发现"),
    ("no_implicit_tools", "保持显式内置工具列表为原生子进程的上限"),
    ("no_mcp", "本次运行禁用 MCP 发现、启动与工具挂载"),
    ("no_migrations", "跳过旧配置 / 会话 / 目录布局的启动迁移"),
    ("no_mouse_capture", "在交互界面中禁用终端鼠标捕获，保留终端原生复制粘贴"),
    ("no_prewalk", "本次启动禁用 prewalk，覆盖设置中的 prewalk.enabled"),
    ("no_prompt_templates", "禁用提示模板发现"),
    ("no_session", "不保存会话（临时会话）"),
    ("no_skills", "禁用技能发现与已配置技能（显式 --skill 路径仍会加载）"),
    ("no_themes", "禁用主题发现"),
    ("no_tools", "禁用全部内置工具"),
    ("plan", "`plan` 角色的模型（计划模式）"),
    ("plan_mode", "以计划模式启动：在计划获批前只做只读规划"),
    ("plan_yolo", "自动批准提交的计划，无需人工复核（无人值守运行）"),
    ("prewalk", "先用初始模型规划，再交给 prewalk.into（默认 @smol）"),
    ("prewalk_into", "覆盖 prewalk.into：provider/model[:thinking] 或显式 @role"),
    ("print", "非交互模式（处理完即退出）"),
    ("prompt_template", "加载提示模板文件或目录（可重复）"),
    ("provider", "LLM 提供方（例如 anthropic、openai、google）；用 --list-providers 查看规范 ID 与别名"),
    ("request_timeout", "请求方 API 的 HTTP 超时（秒）；0 表示不设超时"),
    ("resume", "从会话选择器中选择会话"),
    ("rpc", "以 RPC 模式启动（等价于 --mode rpc）"),
    ("session", "使用指定的会话文件路径"),
    ("session_dir", "会话存储与查找目录"),
    ("session_durability", "会话持久化模式：strict、balanced 或 throughput"),
    ("session_workdir", "恢复前将已有 --session 显式关联到该 workspace"),
    ("skill", "加载技能文件或目录（可重复）"),
    ("slow", "`slow` 角色的模型（深度推理）"),
    ("smol", "`smol` 角色的模型（廉价快速任务，例如子代理扇出）"),
    ("subagent_fallback_policy", "原生子进程的固定路由策略"),
    ("system_prompt", "覆盖系统提示"),
    ("theme", "选择主题（内置名称、已发现主题名或主题 JSON 路径）"),
    ("theme_path", "将主题文件或目录加入发现范围（可重复）"),
    ("thinking", "扩展思考等级"),
    ("tools", "启用指定工具（逗号分隔），覆盖设置中的可选工具开关"),
    ("trust", "信任当前 workspace：允许加载并执行项目内的 .pi/settings.json 与 .pi/extensions"),
    ("verbose", "输出详细的启动过程"),
    ("version", "打印版本信息"),
    ("args", "消息与 @文件引用"),
    ("export", "将会话文件导出为 HTML"),
    ("fetch_models", "从提供方拉取模型列表并写入模型缓存；设置 PI_DISABLE_MODEL_CACHE=1 可跳过缓存"),
    ("list_models", "列出可用模型后退出"),
    ("list_providers", "列出提供方的规范 ID 与别名后退出"),
    ("persist_models", "将验证过的模型目录写入 models.fetched.json"),
    ("profile", "为本次运行启动采样分析器，并将折叠栈写入 <agent-dir>/profiles/"),
    ("refresh_models", "配合 --fetch-models 忽略缓存并强制重新拉取"),
    ("repair_policy", "修复策略模式：off、suggest、auto-safe 或 auto-strict"),
    ("lang", "界面语言：zh-CN（默认）或 en"),
    ("help", "打印帮助"),
    ("wait_async", "打印模式下等待后台任务并让智能体处理其结果"),
    ("warn_cost", "会话树累计花费达到该美元金额时警告一次"),
    ("yolo", "--approval-mode yolo 的简写"),
    ("acp", "以 ACP（Agent Client Protocol）模式启动，供 Zed 编辑器集成"),
    ("mcp_config", "额外的 MCP 服务器配置文件（可重复，优先级最高）"),
];

/// 子命令 id → 中文简介。
#[rustfmt::skip]
const ZH_CLI_SUBCOMMANDS: &[(&str, &str)] = &[
    ("commit", "从暂存改动生成提交"),
    ("completions", "生成 shell 补全脚本"),
    ("config", "查看配置路径与生效值"),
    ("context-preview", "预览发送给模型的上下文"),
    ("doctor", "检查运行环境与配置"),
    ("gc", "清理过期会话与缓存"),
    ("grievances", "查看已记录的缺陷与变通方案"),
    ("handoff", "生成会话交接摘要"),
    ("import", "导入外部会话"),
    ("info", "显示构建与环境信息"),
    ("install", "安装扩展或包"),
    ("list", "列出已安装资源"),
    ("migrate", "迁移旧版配置与会话"),
    ("profile", "分析会话的耗时与调用分布"),
    ("recover", "从损坏的会话中恢复数据"),
    ("remove", "移除已安装资源"),
    ("review", "复核改动"),
    ("rules", "列出项目规则来源"),
    ("search", "搜索扩展与技能"),
    ("self-update", "更新 Pi 到最新版本"),
    ("stats", "统计用量与花费"),
    ("swarm-progress", "查看并行任务进度"),
    ("swarm-replay-preview", "预览并行任务回放"),
    ("token", "管理访问令牌"),
    ("update", "更新已安装资源"),
    ("update-index", "刷新扩展索引"),
    ("usage", "显示用量"),
    ("validation-broker", "运行校验代理"),
    ("web", "启动浏览器伴生界面"),
    ("worktree", "管理 git worktree"),
];

/// 顶层 `about` 文案。
const ZH_CLI_ABOUT: &str = "Pi — AI 编程智能体命令行";

/// 顶层 `after_help` 示例。
const ZH_CLI_AFTER_HELP: &str = "示例：
  pi \"解释这段代码\"                以消息开始新会话
  pi @file.rs \"审查这个文件\"       将文件纳入上下文
  pi -c                              继续上一个会话
  pi -r                              从会话选择器恢复
  pi -p \"2+2 等于几\"               打印模式（非交互）
  pi --model claude-opus-4 \"帮忙\"   使用指定模型";

/// clap 硬编码的分节标题。`Command` 没有提供改写入口，只能在渲染后替换。
#[rustfmt::skip]
const CLAP_HEADINGS: &[(&str, &str)] = &[
    ("Usage:", "用法："),
    ("Commands:", "命令："),
    ("Arguments:", "参数："),
    ("Options:", "选项："),
];

/// clap 自动生成的 help / version 标志说明；同样只能在渲染后替换。
#[rustfmt::skip]
const CLAP_PHRASES: &[(&str, &str)] = &[
    (
        "Print help (see a summary with '-h')",
        "打印帮助（加 -h 查看简要说明）",
    ),
    ("Print help", "打印帮助"),
    ("Print version", "打印版本"),
    (
        "Print this message or the help of the given subcommand(s)",
        "打印本信息，或指定子命令的帮助",
    ),
];

/// 本地化 clap 渲染出的帮助文本。
///
/// 只替换行首分节标题与自动生成标志的说明；参数取值、标识符与用户数据不受影响。
#[must_use]
pub fn localize_clap_output(text: &str) -> String {
    if current() != Locale::ZhCn {
        return text.to_string();
    }
    let mut output = String::with_capacity(text.len() + 16);
    for line in text.lines() {
        let mut rendered = None;
        for (source, translated) in CLAP_HEADINGS {
            if let Some(rest) = line.strip_prefix(source) {
                rendered = Some(format!(
                    "{translated}{}",
                    rest.strip_prefix(' ').unwrap_or(rest)
                ));
                break;
            }
        }
        let mut out = rendered.unwrap_or_else(|| line.to_string());
        for (source, translated) in CLAP_PHRASES {
            if out.contains(source) {
                out = out.replace(source, translated);
            }
        }
        output.push_str(&out);
        output.push('\n');
    }
    if !text.ends_with('\n') {
        output.pop();
    }
    output
}

fn lookup_arg(id: &str) -> Option<&'static str> {
    ZH_CLI_ARGS
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, text)| *text)
}

fn lookup_subcommand(id: &str) -> Option<&'static str> {
    ZH_CLI_SUBCOMMANDS
        .iter()
        .find(|(key, _)| *key == id)
        .map(|(_, text)| *text)
}

/// 就地本地化 clap 命令树（帮助文本、子命令简介）。
///
/// 英文（或未收录条目）保持原样；标识符、取值名与占位符不翻译。
pub fn localize_command(command: clap::Command) -> clap::Command {
    if current() != Locale::ZhCn {
        return command;
    }
    let mut command = command;
    let root_id = command.get_name().to_string();
    if root_id == "pi" {
        command = command
            .about(ZH_CLI_ABOUT)
            .long_about(ZH_CLI_ABOUT)
            .after_help(ZH_CLI_AFTER_HELP);
    }

    let arg_ids = command
        .get_arguments()
        .map(|arg| arg.get_id().to_string())
        .collect::<Vec<_>>();
    for id in arg_ids {
        if let Some(text) = lookup_arg(&id) {
            command = command.mut_arg(id, |arg| arg.help(text).long_help(text));
        }
    }

    let subcommand_ids = command
        .get_subcommands()
        .map(|sub| sub.get_name().to_string())
        .collect::<Vec<_>>();
    for id in subcommand_ids {
        if let Some(text) = lookup_subcommand(&id) {
            command = command.mut_subcommand(id, |sub| sub.about(text).long_about(text));
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_language_tags() {
        assert_eq!(Locale::parse("zh"), Some(Locale::ZhCn));
        assert_eq!(Locale::parse("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(Locale::parse("zh_CN.UTF-8"), Some(Locale::ZhCn));
        assert_eq!(Locale::parse("en-US"), Some(Locale::En));
        assert_eq!(Locale::parse(""), None);
        assert_eq!(Locale::parse("fr"), None);
    }

    #[test]
    fn english_is_identity() {
        for (source, _) in ZH_CATALOG.iter().chain(ZH_UI.iter()).chain(ZH_HINTS.iter()) {
            assert_eq!(tr_in(Locale::En, source), *source);
        }
    }

    #[test]
    fn catalog_entries_are_unique_and_non_empty() {
        let mut seen = std::collections::HashSet::new();
        for (source, translated) in ZH_CATALOG.iter().chain(ZH_UI.iter()).chain(ZH_HINTS.iter()) {
            assert!(!source.is_empty(), "empty catalog key");
            assert!(!translated.is_empty(), "empty translation for {source}");
            assert!(seen.insert(*source), "duplicate catalog key: {source}");
        }
    }

    #[test]
    fn cli_arg_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for (id, _) in ZH_CLI_ARGS {
            assert!(seen.insert(*id), "duplicate CLI arg id: {id}");
        }
        let mut seen = std::collections::HashSet::new();
        for (id, _) in ZH_CLI_SUBCOMMANDS {
            assert!(seen.insert(*id), "duplicate subcommand id: {id}");
        }
    }

    #[test]
    fn cli_arg_ids_exist_in_the_command_tree() {
        use clap::CommandFactory;
        let command = crate::cli::Cli::command();
        let root_ids = command
            .get_arguments()
            .map(|arg| arg.get_id().to_string())
            .collect::<std::collections::HashSet<_>>();
        let subcommand_ids = command
            .get_subcommands()
            .map(|sub| sub.get_name().to_string())
            .collect::<std::collections::HashSet<_>>();
        for (id, _) in ZH_CLI_ARGS {
            assert!(
                root_ids.contains(*id),
                "unknown CLI arg id in catalog: {id}"
            );
        }
        for (id, _) in ZH_CLI_SUBCOMMANDS {
            assert!(
                subcommand_ids.contains(*id),
                "unknown subcommand id in catalog: {id}"
            );
        }
    }

    #[test]
    fn t_format_substitutes_named_placeholders() {
        let rendered = t_format_in(Locale::En, "running {tool} ...", &[("tool", "bash")]);
        assert_eq!(rendered, "running bash ...");
        let rendered = t_format_in(Locale::ZhCn, "running {tool} ...", &[("tool", "bash")]);
        assert_eq!(rendered, "正在运行 bash ...");
    }

    #[test]
    fn unknown_placeholders_are_preserved() {
        let rendered = t_format_in(Locale::En, "a {b} c", &[("z", "1")]);
        assert_eq!(rendered, "a {b} c");
    }
}
