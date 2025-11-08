/**
 * 调试 Vue 应用挂载问题
 */

import { test, expect } from './hooks.js'

test('调试 Vue 挂载', async ({ page }) => {
  const consoleMessages = []
  const pageErrors = []
  const failedRequests = []

  // 捕获所有控制台消息
  page.on('console', (msg) => {
    const text = msg.text()
    consoleMessages.push({ type: msg.type(), text })
    console.log(`[${msg.type()}]`, text)
  })

  // 捕获页面错误
  page.on('pageerror', (error) => {
    pageErrors.push(error.message)
    console.log('❌ Page error:', error.message)
  })

  // 捕获请求失败
  page.on('requestfailed', (request) => {
    const url = request.url()
    const failure = request.failure()
    failedRequests.push({ url, error: failure?.errorText })
    console.log(`⚠️  Request failed: ${url} - ${failure?.errorText}`)
  })

  console.log('\n导航到首页...')
  await page.goto('/', { waitUntil: 'domcontentloaded', timeout: 15000 })

  console.log('\n等待 JavaScript 执行...')
  await page.waitForTimeout(3000)

  // 检查 Vue 是否挂载
  const vueStatus = await page.evaluate(() => {
    const app = document.getElementById('app')
    return {
      appExists: !!app,
      appVisible: app ? window.getComputedStyle(app).display !== 'none' : false,
      appInnerHTML: app?.innerHTML?.substring(0, 500) || '',
      appChildren: app?.children.length || 0,
      windowVue: typeof window.__VUE__ !== 'undefined',
      documentScripts: document.querySelectorAll('script').length,
      documentStyles: document.querySelectorAll('link[rel="stylesheet"], style').length,
      bodyClassName: document.body.className,
      htmlClassName: document.documentElement.className
    }
  })

  console.log('\n=== Vue 状态 ===')
  console.log(JSON.stringify(vueStatus, null, 2))

  console.log('\n=== 控制台消息总数 ===', consoleMessages.length)
  console.log('错误:', consoleMessages.filter((m) => m.type === 'error').length)
  console.log('警告:', consoleMessages.filter((m) => m.type === 'warning').length)

  console.log('\n=== 页面错误总数 ===', pageErrors.length)
  if (pageErrors.length > 0) {
    pageErrors.forEach((err) => console.log('  -', err))
  }

  console.log('\n=== 失败的请求总数 ===', failedRequests.length)
  if (failedRequests.length > 0) {
    failedRequests.slice(0, 10).forEach((req) => console.log('  -', req.url, req.error))
  }

  // 截图
  await page.screenshot({ path: 'playwright-report/vue-mount-debug.png', fullPage: true })

  console.log('\n✓ 调试信息已收集')
})
