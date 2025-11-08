# 系统提示词相似度验证 - 技术设计文档

## 概述

实现 Claude Code 系统提示词相似度验证，用于准确识别真实的 Claude Code 客户端请求。

**参考**: `nodejs-archive/src/utils/contents.js`, `nodejs-archive/src/validators/clients/claudeCodeValidator.js`

## 架构设计

### 1. 核心模块结构

```
rust/src/utils/
├── prompt_similarity/        # 新模块
│   ├── mod.rs               # 模块定义
│   ├── algorithm.rs         # Dice Coefficient 算法
│   ├── templates.rs         # Claude Code 提示词模板
│   ├── normalizer.rs        # 文本标准化
│   └── matcher.rs           # 模板匹配逻辑
└── claude_code_headers.rs   # 现有模块（将集成新功能）
```

### 2. 数据流

```
请求 body.system
    ↓
提取系统提示词文本
    ↓
文本标准化（去空格、处理占位符）
    ↓
与预定义模板比较（Dice Coefficient）
    ↓
计算最佳匹配分数
    ↓
判断: score >= 0.5 ?
    ↓
是真实 Claude Code 请求 / 不是
```

## 技术实现细节

### 1. Dice Coefficient 字符串相似度算法

**公式**:
```
Dice Coefficient = 2 * |X ∩ Y| / (|X| + |Y|)
```

其中 X 和 Y 是两个字符串的 bigram（2-字符组合）集合。

**实现计划**:

```rust
pub fn dice_coefficient(s1: &str, s2: &str) -> f64 {
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    if s1 == s2 {
        return 1.0;
    }

    let bigrams1 = extract_bigrams(s1);
    let bigrams2 = extract_bigrams(s2);

    if bigrams1.is_empty() || bigrams2.is_empty() {
        return 0.0;
    }

    let intersection = bigrams1.intersection(&bigrams2).count();
    let total_bigrams = bigrams1.len() + bigrams2.len();

    2.0 * intersection as f64 / total_bigrams as f64
}

fn extract_bigrams(s: &str) -> HashSet<String> {
    s.chars()
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| format!("{}{}", w[0], w[1]))
        .collect()
}
```

**单元测试计划**:
- 完全相同的字符串 → 1.0
- 完全不同的字符串 → 0.0
- 部分相同的字符串 → 0.0 < score < 1.0
- 空字符串处理
- 单字符字符串处理

### 2. Claude Code 提示词模板

**模板定义**:

```rust
pub struct PromptTemplate {
    pub id: &'static str,
    pub title: &'static str,
    pub category: PromptCategory,
    pub text: &'static str,
}

pub enum PromptCategory {
    System,
    OutputStyle,
    Tools,
    // ... 其他类别
}

// 预定义模板（从 Node.js 代码提取）
pub const CLAUDE_CODE_TEMPLATES: &[PromptTemplate] = &[
    PromptTemplate {
        id: "claude_code_primary",
        title: "Claude Code System Prompt (Primary)",
        category: PromptCategory::System,
        text: "You are Claude Code, Anthropic's official CLI for Claude.",
    },
    PromptTemplate {
        id: "claude_code_secondary",
        title: "Claude Code System Prompt (Secondary)",
        category: PromptCategory::System,
        text: "You are an interactive CLI tool that helps users __PLACEHOLDER__ ...",
        // 完整的长提示词
    },
    // ... 更多模板
];
```

**单元测试计划**:
- 验证所有模板都能正确加载
- 验证模板文本不为空
- 验证模板 ID 唯一性

### 3. 文本标准化

**功能**: 统一文本格式以提高匹配准确性

```rust
pub fn normalize_text(text: &str) -> String {
    // 1. 移除多余空格
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");

    // 2. 替换占位符为单个空格
    let without_placeholder = collapsed.replace("__PLACEHOLDER__", " ");

    // 3. Trim
    without_placeholder.trim().to_string()
}

pub fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
```

**单元测试计划**:
- 多个空格 → 单个空格
- Tab、换行符 → 单个空格
- `__PLACEHOLDER__` → 空格
- 前后空格被 trim

### 4. 占位符处理

**功能**: 处理模板中的 `__PLACEHOLDER__` 标记

```rust
pub const PLACEHOLDER_TOKEN: &str = "__PLACEHOLDER__";

pub fn normalize_for_template(
    value: &str,
    template: &str,
) -> String {
    let parts: Vec<&str> = template.split(PLACEHOLDER_TOKEN).collect();

    if parts.len() <= 1 {
        return normalize_text(value);
    }

    // 如果值完全匹配模板结构（除占位符），返回标准化的模板
    if matches_template_ignoring_placeholders(value, &parts) {
        return normalize_text(template);
    }

    normalize_text(value)
}

fn matches_template_ignoring_placeholders(value: &str, parts: &[&str]) -> bool {
    let value_no_space = value.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let mut cursor = 0;

    for part in parts {
        let part_no_space = part.chars().filter(|c| !c.is_whitespace()).collect::<String>();

        if let Some(pos) = value_no_space[cursor..].find(&part_no_space) {
            cursor += pos + part_no_space.len();
        } else {
            return false;
        }
    }

    true
}
```

**单元测试计划**:
- 完全匹配模板结构 → 返回标准化模板（100%相似度）
- 部分匹配 → 返回标准化输入
- 占位符位置不同 → 正确处理

### 5. 模板匹配主逻辑

```rust
pub struct MatchResult {
    pub best_score: f64,
    pub best_template_id: Option<String>,
    pub threshold: f64,
    pub passed: bool,
}

pub fn best_similarity_by_templates(value: &str) -> MatchResult {
    let threshold = DEFAULT_THRESHOLD; // 0.5
    let normalized_value = normalize_text(value);

    let mut best_score = 0.0;
    let mut best_template_id: Option<String> = None;

    for template in CLAUDE_CODE_TEMPLATES {
        let normalized_template = normalize_text(template.text);

        // 对输入进行模板特定的标准化
        let prepared_value = normalize_for_template(&normalized_value, template.text);

        // 计算相似度
        let score = dice_coefficient(&prepared_value, &normalized_template);

        if score > best_score {
            best_score = score;
            best_template_id = Some(template.id.to_string());
        }
    }

    MatchResult {
        best_score,
        best_template_id,
        threshold,
        passed: best_score >= threshold,
    }
}
```

**单元测试计划**:
- 真实 Claude Code 提示词 → score >= 0.5, passed = true
- 自定义提示词 → score < 0.5, passed = false
- 空字符串 → score = 0.0, passed = false
- 测试所有预定义模板的匹配

### 6. 集成到 claude_code_headers

**更新 `is_real_claude_code_request()`**:

```rust
use crate::utils::prompt_similarity;

pub fn is_real_claude_code_request(request_body: &serde_json::Value) -> bool {
    // 1. 检查 metadata.user_id
    if let Some(metadata) = request_body.get("metadata") {
        if let Some(user_id) = metadata.get("user_id") {
            if let Some(user_id_str) = user_id.as_str() {
                if user_id_str.starts_with("user_")
                    && user_id_str.contains("_account__session_") {
                    return true;
                }
            }
        }
    }

    // 2. 检查系统提示词相似度
    if let Some(system) = request_body.get("system") {
        if let Some(system_array) = system.as_array() {
            for entry in system_array {
                if let Some(text) = entry.get("text") {
                    if let Some(text_str) = text.as_str() {
                        let match_result = prompt_similarity::best_similarity_by_templates(text_str);

                        if match_result.passed {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}
```

## 测试策略

### 1. 单元测试 (rust/src/utils/prompt_similarity/)

**文件**: `algorithm.rs`, `normalizer.rs`, `matcher.rs`, `templates.rs`

**测试内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // algorithm.rs 测试
    #[test]
    fn test_dice_coefficient_identical() {
        let score = dice_coefficient("hello world", "hello world");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_dice_coefficient_different() {
        let score = dice_coefficient("hello", "world");
        assert!(score < 0.5);
    }

    #[test]
    fn test_dice_coefficient_partial() {
        let score = dice_coefficient("hello world", "hello rust");
        assert!(score > 0.0 && score < 1.0);
    }

    // normalizer.rs 测试
    #[test]
    fn test_normalize_text_whitespace() {
        assert_eq!(normalize_text("hello  world"), "hello world");
        assert_eq!(normalize_text("  hello\n\tworld  "), "hello world");
    }

    #[test]
    fn test_normalize_text_placeholder() {
        assert_eq!(
            normalize_text("hello __PLACEHOLDER__ world"),
            "hello world"
        );
    }

    // matcher.rs 测试
    #[test]
    fn test_real_claude_code_prompt() {
        let prompt = "You are Claude Code, Anthropic's official CLI for Claude.";
        let result = best_similarity_by_templates(prompt);
        assert!(result.passed);
        assert!(result.best_score >= 0.5);
    }

    #[test]
    fn test_custom_prompt() {
        let prompt = "You are a helpful assistant.";
        let result = best_similarity_by_templates(prompt);
        assert!(!result.passed);
        assert!(result.best_score < 0.5);
    }
}
```

**运行**: `cargo test --lib prompt_similarity`

### 2. 集成测试 (rust/tests/)

**文件**: `test_system_prompt_validation.rs`

```rust
#[tokio::test]
async fn test_claude_code_request_detection() {
    let request_with_real_prompt = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [],
        "system": [{
            "type": "text",
            "text": "You are Claude Code, Anthropic's official CLI for Claude."
        }]
    });

    assert!(is_real_claude_code_request(&request_with_real_prompt));
}

#[tokio::test]
async fn test_non_claude_code_request_detection() {
    let request_with_custom_prompt = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [],
        "system": [{
            "type": "text",
            "text": "You are a helpful assistant."
        }]
    });

    assert!(!is_real_claude_code_request(&request_with_custom_prompt));
}

#[tokio::test]
async fn test_user_id_detection() {
    let request_with_user_id = json!({
        "model": "claude-3-5-sonnet-20241022",
        "messages": [],
        "metadata": {
            "user_id": "user_1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef_account__session_12345678-1234-1234-1234-123456789012"
        }
    });

    assert!(is_real_claude_code_request(&request_with_user_id));
}
```

**运行**: `bash rust/run-integration-tests.sh`

### 3. E2E 测试

**场景 1**: 真实 Claude Code 请求（完整系统提示词）
- 请求包含完整的 Claude Code 系统提示词
- 验证: 被识别为真实 Claude Code 请求
- 验证: 不添加默认 headers（透传客户端 headers）

**场景 2**: 部分匹配的系统提示词
- 请求包含部分 Claude Code 系统提示词（相似度 0.6）
- 验证: 被识别为真实 Claude Code 请求

**场景 3**: 自定义系统提示词
- 请求包含完全不同的系统提示词
- 验证: 不被识别为 Claude Code 请求
- 验证: 自动添加默认 Claude Code headers

**场景 4**: metadata.user_id 验证
- 请求包含正确格式的 user_id
- 验证: 被识别为真实 Claude Code 请求

## 实施批次

### 批次 1: 核心算法实现（最小可行版本）
- [ ] 实现 Dice Coefficient 算法
- [ ] 实现文本标准化
- [ ] 编写单元测试（算法 + 标准化）
- [ ] 提交: "feat: implement Dice Coefficient string similarity algorithm"

### 批次 2: 模板管理
- [ ] 定义 Claude Code 提示词模板数据结构
- [ ] 实现模板加载和管理
- [ ] 编写单元测试（模板）
- [ ] 提交: "feat: add Claude Code system prompt templates"

### 批次 3: 占位符处理
- [ ] 实现占位符识别和替换逻辑
- [ ] 实现模板结构匹配
- [ ] 编写单元测试（占位符）
- [ ] 提交: "feat: implement placeholder handling for prompt templates"

### 批次 4: 模板匹配主逻辑
- [ ] 实现 best_similarity_by_templates()
- [ ] 集成所有组件
- [ ] 编写单元测试（完整匹配）
- [ ] 提交: "feat: implement prompt template matching logic"

### 批次 5: 集成到 Claude Code headers
- [ ] 更新 is_real_claude_code_request()
- [ ] 添加日志记录
- [ ] 编写集成测试
- [ ] 提交: "feat: integrate prompt similarity validation into Claude Code headers"

### 批次 6: E2E 测试和文档
- [ ] 编写 E2E 测试脚本
- [ ] 运行完整测试套件
- [ ] 更新文档
- [ ] 提交: "docs: add system prompt similarity validation documentation"

## 性能考虑

1. **缓存标准化结果**: 模板的标准化文本可以预计算并缓存
2. **Early exit**: 如果找到完美匹配（score = 1.0），立即返回
3. **Bigram 集合复用**: 对于固定模板，bigram 集合可以预计算

## 配置选项

```rust
pub struct SimilarityConfig {
    pub threshold: f64,           // 默认 0.5
    pub enable_logging: bool,     // 默认 true
}

impl Default for SimilarityConfig {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            enable_logging: true,
        }
    }
}
```

## 向后兼容性

- 现有的 `is_real_claude_code_request()` 仍然工作
- 只是增强了检测逻辑（从简单的 user_id 检查 → user_id + 系统提示词相似度）
- 不影响现有功能

## 文档更新

需要更新的文档：
1. `docs/guides/api-reference.md` - API 验证说明
2. `docs/architecture/overview.md` - 架构图更新
3. `CLAUDE.md` - 添加新模块说明
4. README（如果需要）

## 成功标准

✅ 所有单元测试通过（覆盖率 > 90%）
✅ 所有集成测试通过
✅ E2E 测试通过（4个场景）
✅ 真实 Claude Code 请求被正确识别
✅ 自定义请求被正确拒绝/添加 headers
✅ 性能: 相似度计算 < 5ms
✅ 文档完整且准确
