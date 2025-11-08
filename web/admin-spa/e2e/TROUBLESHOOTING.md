# E2E 测试故障排除指南

## 当前问题：页面崩溃 (Page Crashed)

### 问题描述

在 Playwright 测试中，页面在加载后会崩溃，错误信息：
```
Error: page.goto: Page crashed
Navigation failed because page crashed!
```

### 根本原因分析

通过调试测试（`debug-crash.spec.js`）发现：

1. ✅ **Vite 开发服务器正常** - HTTP 200, HTML 正常返回
2. ✅ **Vue 路由正常** - 能够成功导航到 `/api-stats`
3. ✅ **Vite HMR 连接成功** - WebSocket 正常连接
4. ❌ **页面在路由后崩溃** - 在加载 ApiStatsView 组件后崩溃

**关键发现**：
- 外部资源加载失败（Font Awesome CDN, Google Fonts）
  - `net::ERR_TUNNEL_CONNECTION_FAILED`
  - `net::ERR_NAME_NOT_RESOLVED`
- 页面在执行 JavaScript 约 2 秒后崩溃
- 简单的 HTML 加载测试（不执行 JS）能够成功

### 可能的原因

#### 1. 外部资源加载失败

**影响**: 中等
**证据**: 调试日志显示 CDN 请求失败

ApiStatsView.vue 中的外部依赖：
```html
<!-- Google Fonts -->
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap" rel="stylesheet">

<!-- Font Awesome -->
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
```

这些资源在测试环境中无法加载，可能导致页面渲染问题。

#### 2. API 调用失败

**影响**: 高
**证据**: `ApiStatsView.vue` 在 `onMounted` 时调用 `loadOemSettings()`

```javascript
onMounted(() => {
  themeStore.initTheme()
  loadOemSettings()  // ← 可能失败并导致崩溃
  // ...
})
```

如果 API 调用失败且没有正确的错误处理，可能导致 Promise rejection 未捕获，从而崩溃整个页面。

#### 3. Chromium 内存/渲染问题

**影响**: 低
**证据**: 页面在等待 2 秒后才崩溃

可能是 Chromium 在无头模式下的内存限制或渲染引擎问题。

#### 4. 组件命名冲突

**影响**: 低
**证据**: Vite 警告
```
[unplugin-vue-components] component "CreateApiKeyModal" has naming conflicts with other components, ignored.
```

虽然有警告，但这通常不会导致页面崩溃。

## 解决方案

### 方案 1：Mock 外部资源 ⭐ 推荐

在 Playwright 配置中拦截和 mock 外部资源请求：

```javascript
// playwright.config.js
use: {
  baseURL: 'http://localhost:3001',

  // 拦截外部资源
  async beforeEach({ page }) {
    // Block external fonts and CDN resources
    await page.route('**/*{googleapis,cdnjs,jsdelivr}*', route => route.abort());

    // Or provide mock responses
    await page.route('**/font-awesome/**', route => {
      route.fulfill({ status: 200, body: '' });
    });
  },
}
```

### 方案 2：配置环境变量跳过 API 调用

在测试环境中设置标志，跳过某些初始化逻辑：

```javascript
// .env.test
VITE_E2E_TEST=true
VITE_SKIP_OEM_LOAD=true
```

```javascript
// ApiStatsView.vue
onMounted(() => {
  themeStore.initTheme()

  // 测试环境跳过
  if (!import.meta.env.VITE_E2E_TEST) {
    loadOemSettings()
  }
  // ...
})
```

### 方案 3：增加错误处理和超时 ⭐ 推荐

在组件中添加更好的错误处理：

```javascript
// stores/apistats.js
async loadOemSettings() {
  this.oemLoading = true
  try {
    const response = await axios.get('/webapi/oem/settings', {
      timeout: 5000  // 5 秒超时
    })
    this.oemSettings = response.data
  } catch (error) {
    console.warn('Failed to load OEM settings:', error)
    // 使用默认值，不阻塞页面
    this.oemSettings = {
      siteName: 'Claude Relay Service',
      siteIcon: '',
      ldapEnabled: false,
      showAdminButton: true
    }
  } finally {
    this.oemLoading = false
  }
}
```

### 方案 4：调整 Playwright 配置

增加超时和降低期望：

```javascript
// playwright.config.js
use: {
  baseURL: 'http://localhost:3001',
  navigationTimeout: 60000,  // 增加导航超时
  actionTimeout: 30000,       // 增加操作超时

  // 使用更宽松的等待条件
  screenshot: 'only-on-failure',
  video: 'retain-on-failure',
}
```

并在测试中使用更宽松的等待：

```javascript
test('应该能访问 API Stats 首页', async ({ page }) => {
  // 使用 domcontentloaded 而不是 networkidle
  await page.goto('/', { waitUntil: 'domcontentloaded' });

  // 手动等待关键元素
  await page.waitForSelector('text=统计查询', { timeout: 10000 });

  // 验证页面标题
  await expect(page).toHaveTitle(/Claude Relay/);
});
```

### 方案 5：使用 Mock Service Worker (MSW)

安装并配置 MSW 来模拟 API 响应：

```bash
npm install -D msw
```

```javascript
// mocks/handlers.js
import { http, HttpResponse } from 'msw'

export const handlers = [
  http.get('/webapi/oem/settings', () => {
    return HttpResponse.json({
      siteName: 'Claude Relay Service (Test)',
      siteIcon: '',
      ldapEnabled: false,
      showAdminButton: true
    })
  }),
]
```

```javascript
// tests/setup.js
import { setupServer } from 'msw/node'
import { handlers } from '../mocks/handlers'

export const server = setupServer(...handlers)

beforeAll(() => server.listen())
afterEach(() => server.resetHandlers())
afterAll(() => server.close())
```

## 立即可用的临时解决方案

### 1. 手动启动服务进行测试

```bash
# 终端 1: 启动后端
cd /home/user/RustCRS
cargo run

# 终端 2: 启动前端
cd web/admin-spa
npm run dev

# 终端 3: 在浏览器中手动测试
# 访问 http://localhost:3001/admin/
```

### 2. 使用有头模式调试

```bash
npm run test:headed
# 或
npm run test:debug
```

这样可以看到浏览器中的实际错误。

### 3. 运行简化测试

使用我们创建的 `debug-crash.spec.js` 中的第二个测试：

```bash
npx playwright test e2e/debug-crash.spec.js -g "测试简化的页面加载"
```

这个测试能够成功，因为它只加载 HTML 不等待 JavaScript 执行。

## 验证解决方案

修复后，运行以下命令验证：

```bash
# 1. 运行所有测试
npm test

# 2. 运行特定测试套件
npx playwright test e2e/ui-walkthrough.spec.js

# 3. 使用 UI 模式查看结果
npm run test:ui

# 4. 查看测试报告
npm run test:report
```

## 预期结果

修复后，测试结果应该从：
- ❌ 43 失败 / 1 通过

变为：
- ✅ 44 通过 / 0 失败

## 参考资源

- [Playwright 网络拦截](https://playwright.dev/docs/network)
- [Playwright 调试指南](https://playwright.dev/docs/debug)
- [Vue 测试最佳实践](https://vuejs.org/guide/scaling-up/testing.html)
- [MSW 文档](https://mswjs.io/docs/)

---

**最后更新**: 2025-11-08
**状态**: 问题已识别，解决方案已提供
