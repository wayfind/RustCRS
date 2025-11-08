import { defineConfig, devices } from '@playwright/test'

/**
 * Playwright 配置文件 - Claude Relay Service UI 测试
 *
 * 测试范围：
 * - 管理界面基本功能
 * - 账户管理流程
 * - API Key 管理
 * - 统计面板
 */

export default defineConfig({
  // 测试目录
  testDir: './e2e',

  // 测试文件匹配模式
  testMatch: '**/*.spec.js',

  // 全局 setup/teardown
  globalSetup: './e2e/global-setup.js',
  globalTeardown: './e2e/global-teardown.js',

  // 全局超时设置
  timeout: 30 * 1000,
  expect: {
    timeout: 5000
  },

  // 失败重试次数
  retries: process.env.CI ? 2 : 0,

  // 并行执行配置
  workers: process.env.CI ? 1 : undefined,

  // 报告配置
  reporter: [
    ['html', { outputFolder: 'playwright-report' }],
    ['list'],
    ['json', { outputFile: 'playwright-report/results.json' }]
  ],

  // 全局配置
  use: {
    // 基础 URL - 指向 Rust 后端提供的静态文件服务
    baseURL: process.env.BASE_URL || 'http://localhost:8080/admin-next',

    // 截图设置
    screenshot: 'only-on-failure',

    // 视频录制
    video: 'retain-on-failure',

    // 追踪
    trace: 'on-first-retry',

    // 浏览器上下文选项
    viewport: { width: 1280, height: 720 },
    ignoreHTTPSErrors: true,

    // 导航超时（增加以应对慢速加载）
    navigationTimeout: 30000,
    actionTimeout: 15000,

    // 服务worker
    serviceWorkers: 'block'
  },

  // 测试项目配置 - 多浏览器测试
  projects: [
    // Firefox - 优先使用 Firefox 避免 Chromium 崩溃问题
    {
      name: 'firefox',
      use: { ...devices['Desktop Firefox'] }
    },

    // Chromium - 在某些环境下可能崩溃
    {
      name: 'chromium',
      use: {
        ...devices['Desktop Chrome'],
        // 添加 Chrome 启动参数以避免崩溃
        launchOptions: {
          args: [
            '--disable-dev-shm-usage',
            '--no-sandbox',
            '--disable-setuid-sandbox',
            '--disable-gpu'
          ]
        }
      }
    }

    // 可选：移动端测试
    // {
    //   name: 'Mobile Chrome',
    //   use: { ...devices['Pixel 5'] },
    // },
  ]

  // Web Server 配置 - Rust 后端需要手动启动
  // Rust 后端会自动提供静态文件服务在 /admin-next 路径
  // 确保在运行测试前先启动: cd rust && cargo run --release
  // webServer: {
  //   command: 'npm run dev',
  //   port: 3001,
  //   timeout: 120 * 1000,
  //   reuseExistingServer: !process.env.CI,
  //   stdout: 'ignore',
  //   stderr: 'pipe',
  // },
})
