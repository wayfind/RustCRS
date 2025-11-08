# E2E UI 测试指南

本目录包含使用 Playwright 编写的端到端 UI 测试。

## 测试文件

| 文件 | 说明 |
|------|------|
| `ui-walkthrough.spec.js` | UI 漫游测试 - 公开页面、导航、主题切换、响应式设计、性能、可访问性 |
| `admin-auth.spec.js` | 管理员认证测试 - 登录表单、验证、路由保护、安全性 |
| `api-stats.spec.js` | API 统计页面测试 - 数据展示、图表、交互、响应式布局 |

## 快速开始

### 前置要求

确保后端服务正在运行：

```bash
# 方式 1: 使用快速启动脚本
cd /home/user/RustCRS
bash start-dev.sh

# 方式 2: 手动启动
# 1. 启动 Redis
docker run -d --name redis-dev -p 6379:6379 redis:7-alpine

# 2. 启动 Rust 后端
cargo run
```

### 安装 Playwright 浏览器

首次运行前需要安装浏览器：

```bash
cd web/admin-spa
npx playwright install
```

### 运行测试

```bash
cd web/admin-spa

# 运行所有测试（无头模式）
npm test

# 带 UI 界面运行
npm run test:ui

# 有头模式（可以看到浏览器）
npm run test:headed

# 调试模式
npm run test:debug

# 运行特定测试文件
npx playwright test e2e/ui-walkthrough.spec.js

# 运行特定测试用例
npx playwright test -g "应该能访问 API Stats 首页"
```

### 查看测试报告

```bash
npm run test:report
```

## 测试覆盖范围

### ✅ 已实现

1. **UI 漫游测试** (`ui-walkthrough.spec.js`)
   - ✅ 公开页面访问（API Stats 首页）
   - ✅ 导航测试（公开路由）
   - ✅ 主题切换（暗黑模式）
   - ✅ 响应式设计（移动端、平板、桌面）
   - ✅ 性能测试（加载时间、页面切换）
   - ✅ 可访问性测试（语义化标签、键盘导航）
   - ✅ 错误处理（404 重定向、网络错误）

2. **管理员认证测试** (`admin-auth.spec.js`)
   - ✅ 登录页面展示
   - ✅ 表单验证
   - ✅ 表单交互（输入、Tab 导航、Enter 提交）
   - ✅ 路由保护（未认证重定向）
   - ✅ 密码字段安全性
   - ✅ UI/UX（品牌标识、页面标题、间距）

3. **API 统计页面测试** (`api-stats.spec.js`)
   - ✅ 页面加载
   - ✅ 数据展示（统计卡片、数字指标）
   - ✅ 图表渲染（Canvas/SVG）
   - ✅ 交互功能（刷新、时间范围、筛选）
   - ✅ 响应式布局
   - ✅ 数据准确性（数字验证、百分比范围）
   - ✅ 空状态处理

### ⏳ 待实现

- 管理员认证后的完整流程（需要测试账户）
- 账户管理功能
- API Key 管理功能
- 设置页面
- 用户管理
- 数据导出功能
- WebSocket 实时更新

## 测试架构

### 配置文件

- `playwright.config.js` - Playwright 主配置
  - 测试目录: `./e2e`
  - 超时: 30 秒
  - 重试: CI 环境 2 次
  - 报告: HTML + List + JSON
  - 浏览器: Chromium (默认)
  - Web Server: 自动启动 `npm run dev` (端口 3001)

### 测试策略

1. **公开页面优先**: 先测试无需认证的功能
2. **渐进式测试**: 从简单到复杂
3. **容错性**: 使用 `.or()` 和条件检查处理不同 UI 实现
4. **信息性日志**: 使用 `console.log` 记录测试过程

### 测试模式

```javascript
// 宽松模式 - 适合 UI 结构不确定的情况
const element = page.locator('.primary-selector')
  .or(page.locator('.fallback-selector'));

// 条件测试 - 根据元素是否存在决定测试
if (await element.isVisible()) {
  // 执行测试
} else {
  console.log('⚠ 元素不存在，跳过测试');
}
```

## 最佳实践

### 1. 等待策略

```javascript
// ✅ 推荐：等待网络空闲
await page.waitForLoadState('networkidle');

// ✅ 等待特定元素
await page.waitForSelector('.data-loaded');

// ⚠️ 谨慎使用：固定时间等待
await page.waitForTimeout(1000); // 仅用于动画等场景
```

### 2. 选择器优先级

```javascript
// 1. 优先使用角色和文本
page.getByRole('button', { name: '登录' })
page.getByText('用户名')

// 2. 使用测试 ID
page.locator('[data-testid="login-button"]')

// 3. 使用 CSS 选择器（作为后备）
page.locator('.login-button')
```

### 3. 断言最佳实践

```javascript
// ✅ 使用 expect 的 await
await expect(element).toBeVisible();

// ✅ 使用超时
await expect(element).toBeVisible({ timeout: 5000 });

// ✅ 软断言（不会立即失败）
await expect.soft(element).toBeVisible();
```

### 4. 测试隔离

```javascript
test.describe('功能模块', () => {
  test.beforeEach(async ({ page }) => {
    // 每个测试前重置状态
    await page.context().clearCookies();
    await page.goto('/');
  });

  test('测试用例 1', async ({ page }) => {
    // 测试逻辑
  });
});
```

## 调试技巧

### 1. UI 模式

```bash
npm run test:ui
```

最直观的调试方式，可以：
- 查看每个测试步骤
- 时间旅行（回放）
- 检查选择器
- 查看网络请求

### 2. 调试模式

```bash
npm run test:debug
```

逐步执行，可以：
- 在浏览器中暂停
- 使用 DevTools
- 手动执行命令

### 3. 截图和视频

测试失败时会自动截图和录像，位置：
- 截图: `test-results/`
- 视频: `test-results/`
- 报告: `playwright-report/`

### 4. 代码生成

```bash
npm run test:codegen
```

自动录制操作生成测试代码

## CI/CD 集成

### GitHub Actions 示例

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install dependencies
        run: |
          cd web/admin-spa
          npm ci

      - name: Install Playwright browsers
        run: npx playwright install --with-deps

      - name: Start backend
        run: |
          # 启动 Redis 和 Rust 后端
          docker run -d --name redis-dev -p 6379:6379 redis:7-alpine
          cargo run &
          sleep 10

      - name: Run Playwright tests
        run: |
          cd web/admin-spa
          npm test

      - name: Upload test results
        if: always()
        uses: actions/upload-artifact@v3
        with:
          name: playwright-report
          path: web/admin-spa/playwright-report/
```

## 常见问题

### 1. 测试超时

```bash
Error: Test timeout of 30000ms exceeded
```

解决方案：
- 增加超时时间: `test.setTimeout(60000)`
- 检查网络请求是否卡住
- 使用 `waitForLoadState('networkidle')`

### 2. 元素未找到

```bash
Error: Locator resolved to 0 elements
```

解决方案：
- 使用 `page.pause()` 检查页面状态
- 使用 `.or()` 提供备选选择器
- 添加等待: `await page.waitForSelector()`

### 3. 端口冲突

```bash
Error: Port 3001 already in use
```

解决方案：
```bash
# 查找占用端口的进程
lsof -i :3001

# 终止进程
kill -9 <PID>
```

### 4. 浏览器未安装

```bash
Error: Executable doesn't exist
```

解决方案：
```bash
npx playwright install
```

## 性能优化

### 1. 并行执行

```javascript
// playwright.config.js
export default defineConfig({
  workers: 4, // 4 个并行 worker
});
```

### 2. 复用认证状态

```javascript
// auth.setup.js
test('setup', async ({ page }) => {
  await page.goto('/login');
  // 登录...
  await page.context().storageState({ path: 'auth.json' });
});

// 测试中使用
test.use({ storageState: 'auth.json' });
```

### 3. 跳过不必要的资源

```javascript
await page.route('**/*.{png,jpg,jpeg,gif,svg}', route => route.abort());
```

## 贡献指南

添加新测试时：

1. 遵循命名规范: `功能模块.spec.js`
2. 添加清晰的测试描述
3. 使用 `test.describe` 分组相关测试
4. 添加必要的注释
5. 更新本 README

## 资源链接

- [Playwright 官方文档](https://playwright.dev)
- [Playwright 最佳实践](https://playwright.dev/docs/best-practices)
- [Element Plus 组件](https://element-plus.org)
- [项目架构文档](../../../CLAUDE.md)

---

**最后更新**: 2025-11-08
**维护者**: Claude Relay Service Team
