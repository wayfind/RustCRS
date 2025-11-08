/**
 * 测试 Hooks - 在每个测试之前/之后运行
 *
 * 用法：在测试文件中导入并使用
 */

import { test as base } from '@playwright/test'

/**
 * 扩展的 test 对象，包含自动资源拦截
 */
export const test = base.extend({
  page: async ({ page }, use) => {
    // 在每个测试之前：拦截外部资源
    await page.route('**/*{googleapis,gstatic,cdnjs,jsdelivr,cloudflare}*/**', (route) => {
      const url = route.request().url()

      // CDN 字体和样式
      if (url.includes('fonts.googleapis') || url.includes('cdnjs.cloudflare')) {
        route.fulfill({
          status: 200,
          contentType: 'text/css',
          body: '/* Mocked CSS */'
        })
      }
      // Google Fonts 字体文件
      else if (url.includes('fonts.gstatic')) {
        route.fulfill({
          status: 200,
          contentType: 'font/woff2',
          body: ''
        })
      }
      // 其他外部资源
      else {
        route.fulfill({
          status: 200,
          body: ''
        })
      }
    })

    // 捕获页面错误
    page.on('pageerror', (error) => {
      console.log(`❌ Page error: ${error.message}`)
    })

    // 捕获控制台错误（仅在调试时）
    if (process.env.DEBUG) {
      page.on('console', (msg) => {
        if (msg.type() === 'error') {
          console.log(`🔴 Console error: ${msg.text()}`)
        }
      })
    }

    // 捕获请求失败（仅在调试时）
    if (process.env.DEBUG) {
      page.on('requestfailed', (request) => {
        console.log(`⚠️  Request failed: ${request.url()} - ${request.failure()?.errorText}`)
      })
    }

    // 使用 page
    await use(page)

    // 测试后清理（如果需要）
  }
})

export { expect } from '@playwright/test'

/**
 * 辅助函数：等待 Vue 应用就绪
 */
export async function waitForVueApp(page) {
  // 等待 Vite HMR 连接
  await page.waitForFunction(
    () => {
      return (
        window.__vite_plugin_checker_notification_api__ !== undefined ||
        document.querySelector('[data-v-app]') !== null
      )
    },
    { timeout: 10000 }
  )
}

/**
 * 辅助函数：等待 API 调用完成
 */
export async function waitForApiCalls(page) {
  // 等待所有进行中的 fetch 请求完成
  await page.waitForLoadState('networkidle', { timeout: 10000 })
}
