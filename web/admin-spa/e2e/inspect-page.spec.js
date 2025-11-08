/**
 * 检查页面实际渲染内容
 */

import { test, expect } from './hooks.js'

test('检查首页实际内容', async ({ page }) => {
  await page.goto('/')
  await page.waitForLoadState('networkidle')

  // 截图
  await page.screenshot({ path: 'playwright-report/homepage-debug.png', fullPage: true })

  // 获取页面 HTML
  const html = await page.content()
  console.log('\n=== 页面 HTML (前 2000 字符) ===')
  console.log(html.substring(0, 2000))

  // 获取页面文本内容
  const bodyText = await page.textContent('body')
  console.log('\n=== 页面文本内容 ===')
  console.log(bodyText)

  // 查找所有元素
  const allElements = await page.$$('*')
  console.log(`\n=== 页面元素总数: ${allElements.length} ===`)

  // 查找特定类名
  const glassElements = await page.$$('.glass-strong')
  console.log(`找到 .glass-strong 元素: ${glassElements.length} 个`)

  // 检查是否有错误消息
  const errorElements = await page.$$('[role="alert"], .error, .el-message-box')
  console.log(`找到错误消息元素: ${errorElements.length} 个`)

  // 检查 Vue 应用是否挂载
  const vueApp = await page.evaluate(() => {
    return {
      hasVueApp: !!document.querySelector('#app'),
      appContent: document.querySelector('#app')?.innerHTML?.substring(0, 500),
      htmlClasses: document.documentElement.className,
      bodyClasses: document.body.className
    }
  })
  console.log('\n=== Vue 应用状态 ===')
  console.log(JSON.stringify(vueApp, null, 2))
})
