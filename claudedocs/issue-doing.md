# 修复中问题 (IN PROGRESS Issues)

**更新时间**: 2025-11-03
**状态**: 🟡 进行中

---

## 📋 使用说明

### 文件职责
- **issue-todo.md**: 问题从该文件移动过来
- **本文件**: 记录所有正在修复的问题
- **issue-done.md**: 问题修复完成后移动到该文件

### 问题流转
```
issue-todo.md (待修复)
    → 本文件 (修复中) ✅ 当前阶段
        → issue-done.md (已完成)
```

### 批次修复规则
- ✅ 每批修复 ≤5 个问题
- ✅ 相关问题放在同一批次
- ✅ 按照依赖树从底层到表层修复
- ✅ 每批修复后运行完整测试套件
- ✅ 每批修复后检查接口文档完整性
- ✅ 每批修复后补充集成测试用例

---

## 🎯 当前工作批次

### 批次 15 (已完成): SPA 路由支持修复

**包含问题**: 1 个 (P0 × 1)
- ✅ ISSUE-UI-015: SPA 子路径返回 404 错误 (P0 - Critical)

**开始时间**: 2025-11-05
**完成时间**: 2025-11-05
**状态**: ✅ 已完成

**修复内容**:
1. ✅ 修改 `main.rs:8` - 添加 `ServeFile` 导入
2. ✅ 修改 `main.rs:219-222` - 实现 SPA fallback 机制
3. ✅ 验证所有子路径正常工作（dashboard, accounts, api-keys）
4. ✅ UI 回归测试通过

**技术实现**:
```rust
// rust/src/main.rs:8
use tower_http::services::{ServeDir, ServeFile};

// rust/src/main.rs:219-222
// SPA fallback: serve index.html for all unmatched routes
let index_path = static_dir.join("index.html");
let serve_dir = ServeDir::new(&static_dir)
    .not_found_service(ServeFile::new(&index_path));
```

**验证结果**:
- ✅ `/admin-next/dashboard` - Dashboard 页面完全正常
- ✅ `/admin-next/accounts` - 账户管理页面完全正常
- ✅ `/admin-next/api-keys` - API Keys 管理页面完全正常
- ✅ 页面刷新功能正常
- ✅ 直接访问子路径正常
- ✅ 静态资源（JS、CSS）加载正常

**集成测试**:
- ✅ 创建 7 个测试用例覆盖所有 SPA 路由场景
- ✅ 所有测试通过（7 passed / 0 failed）
- ✅ 测试文件: `rust/tests/test_spa_routing.rs`

**详细报告**: `claudedocs/batch-15-spa-routing-fix.md`

---

## 📜 已完成批次

### 批次 13 (已完成): 标签和账户管理功能修复

**包含问题**: 2 个 (P2 × 2)
- ✅ ISSUE-UI-006: 创建 API Key 时设置的标签未显示 (P2)
- ✅ ISSUE-UI-011: 添加账户对话框打开时 404 错误 (P2)

**完成时间**: 2025-11-03
**状态**: ✅ 已完成

**修复内容**:
1. ISSUE-UI-006: 为 `ApiKeyRequest` 添加 `tags` 字段，在创建时正确传递标签
2. ISSUE-UI-011: 实现 `/admin/claude-code-version` 和 `/admin/claude-code-version/clear` 端点

**验证结果**:
- ✅ 单元测试: 107个全部通过
- ✅ 集成测试: 3个标签测试全部通过
- ✅ 编译测试通过，端点已注册
- ✅ 向后兼容性良好

**详细报告**: `claudedocs/batch-13-completion-report.md`

---

### 批次 12 (已完成): API Key 删除功能修复

**包含问题**: 1 个 (P0)
- ✅ ISSUE-UI-008: 删除 API Key 操作未生效 (P0 - Critical)

**完成时间**: 2025-11-03
**状态**: ✅ 已完成

**修复内容**:
1. 修复 API Key 状态字段序列化格式（添加 serde rename 属性）
2. 字段命名从 snake_case 改为 camelCase (isActive, isDeleted, deletedBy, deletedByType)
3. 新增集成测试 (`rust/tests/test_api_key_delete.rs`)

**验证结果**:
- ✅ 单元测试: 107个全部通过
- ✅ 集成测试: 2个新增测试全部通过
- ✅ 删除逻辑本身正确，问题在序列化层
- ✅ 字段名修复完成，camelCase 兼容

**详细报告**: `claudedocs/batch-12-completion-report.md`

---

### 批次 11 (已完成): Tags 端点别名和日期格式修复

**包含问题**: 2 个 (P1 × 1, P2 × 1)
- ✅ ISSUE-UI-004: GET /admin/tags 返回 405 Method Not Allowed (P1) - 批次 9 已修复
- ✅ ISSUE-UI-005: API Key 创建时间显示 "Invalid Date" (P2)

**完成时间**: 2025-11-03
**状态**: ✅ 已完成

**修复内容**:
1. 验证 GET /admin/tags 端点（批次 9 已修复）
2. 修复 API Key 日期字段序列化格式（添加 serde rename 属性）
3. 新增集成测试 (`rust/tests/test_api_key_date_format.rs`)

**验证结果**:
- ✅ 单元测试: 107个全部通过
- ✅ 集成测试: 日期格式测试全部通过
- ✅ ISSUE-UI-004: 已在批次 9 修复并验证
- ✅ ISSUE-UI-005: 字段名修复完成，camelCase 兼容

**详细报告**: `claudedocs/batch-11-completion-report.md`

---

### 批次 10 (已完成): API Keys 编辑和创建功能修复

**包含问题**: 3 个 (P2 × 3)
- ✅ ISSUE-UI-009: 编辑 API Key 时获取详情失败 (404)
- ✅ ISSUE-UI-007: 编辑 API Key 后名称未更新
- ✅ ISSUE-UI-010: 创建 API Key 成功后 JavaScript 错误

**完成时间**: 2025-11-03
**状态**: ✅ 已完成

**修复内容**:
1. 实现 GET /admin/api-keys/:id 端点
2. 修复响应字段一致性（统一使用 `data` 字段）
3. 新增集成测试 (`rust/tests/test_api_key_detail.rs`)

**验证结果**:
- ✅ 单元测试: 107个全部通过
- ✅ 集成测试: 5个新增测试全部通过
- ✅ UI 漫游测试: 编辑和保存功能正常
- ⚠️ API 文档待更新

**详细报告**: `claudedocs/batch-10-completion-report.md`

---

### 批次 8 (已完成): Dashboard 数据结构修复

**包含问题**: 1 个 (P0)
- ✅ ISSUE-UI-003: Dashboard 数据字段名不匹配导致前端报错

**完成时间**: 2025-11-03
**状态**: ✅ 已完成

**修复内容**:
1. 修复 `/admin/model-stats` 响应字段 (`models` → `data`)
2. 重写 `/admin/dashboard` 返回完整数据结构
3. 修复单元测试中的函数调用

**验证结果**:
- ✅ 单元测试: 107个全部通过
- ✅ Dashboard 页面正常加载,无 JavaScript 错误
- ✅ API 文档已更新

---

## 🔄 工作流程提醒

1. **修复前**: 编写失败的集成测试 (TDD)
2. **修复中**: 实现功能,运行测试验证
3. **修复后**:
   - 运行完整测试套件 (`cargo test --lib`)
   - UI 漫游测试验证
   - 更新 API 文档
   - 创建批次完成报告
   - 提交到 git
4. **完成**: 移动问题到 issue-done.md
