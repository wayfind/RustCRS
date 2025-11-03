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

**批次 9**: API Keys 删除功能和标签管理

**包含问题**: 2 个 (P0 × 1, P1 × 1)
- ISSUE-UI-008: 删除 API Key 操作未生效 (P0)
- ISSUE-UI-004: GET /admin/tags 返回 405 Method Not Allowed (P1)

**开始时间**: 2025-11-03
**状态**: 🔄 修复中

---

## 📝 问题详情

### ISSUE-UI-008 - 删除 API Key 操作未生效

**优先级**: P0
**模块**: 管理后台/API Keys/删除功能
**状态**: ✅ 已验证 - **非实际问题**

**问题描述**:
点击删除按钮并确认后,显示"API Key 已删除"成功提示,但 API Key 仍然在活跃列表中显示,删除操作完全未生效。

**调查结果**:
通过代码审查和 UI 测试验证,删除功能**完全正常工作**:

1. ✅ **代码审查** (admin.rs:560, api_key.rs:387, api_key.rs:318):
   - 删除处理器正确调用 ApiKeyService::delete_key
   - 软删除正确设置 is_deleted=true
   - 列表查询正确过滤已删除项

2. ✅ **UI 测试验证** (Playwright):
   - 删除操作成功: 显示"API Key 已删除"消息
   - API Key 从活跃列表移除: 3 → 2 条记录
   - 计数正确更新: "活跃 API Keys 3" → "活跃 API Keys 2"

**结论**: **ISSUE-UI-008 是误报** - 删除功能实际运行正常,无需修复。

**新发现问题**:
在测试过程中发现 GET `/admin/api-keys/deleted` 返回 405 错误,"已删除 API Keys"标签页无法加载。这是一个独立的问题,不影响删除操作本身。

---

### ISSUE-UI-004 - GET /admin/tags 返回 405 Method Not Allowed

**优先级**: P1
**模块**: 管理后台/API Keys/标签管理
**状态**: ✅ 已修复

**问题描述**:
创建或编辑 API Key 时,前端请求 GET /admin/tags 返回 405 Method Not Allowed,标签下拉列表无法加载已有标签。

**根本原因**:
标签端点已实现在 `/admin/api-keys/tags`,但前端请求 `/admin/tags`,路由不匹配导致 405 错误。

**修复方案**:
1. ✅ 在 admin.rs:188 添加路由别名 `.route("/tags", get(get_api_keys_tags_handler))`
2. ✅ 保持原路由 `/api-keys/tags` 以实现向后兼容
3. ✅ 编写集成测试 `test_get_tags_endpoint` - 测试通过
4. ✅ 更新 API 文档添加别名说明

**修复文件**: `rust/src/routes/admin.rs:188`

**集成测试**: `rust/tests/test_get_tags_endpoint.rs` ✅ 通过

**验证结果**:
- ✅ 编译成功
- ✅ 集成测试通过 (3 passed)
- ✅ API 文档已更新

---

## 📜 已完成批次

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
