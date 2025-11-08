/**
 * 简单的页面加载检查
 */

import { test, expect } from './hooks.js'

test('简单检查 - 不使用 networkidle', async ({ page }) => {
  console.log('正在导航到首页...')
  await page.goto('/', { waitUntil: 'domcontentloaded' })

  console.log('等待 #app 元素...')
  await page.waitForSelector('#app', { timeout: 10000 })

  console.log('获取页面内容...')
  const appContent = await page.locator('#app').innerHTML()
  console.log('App 内容长度:', appContent.length)
  console.log('App 内容 (前 500 字符):', appContent.substring(0, 500))

  // 截图
  await page.screenshot({ path: 'playwright-report/simple-check.png' })

  // 获取所有文本内容
  const bodyText = await page.textContent('body')
  console.log('\n页面文本:', bodyText?.substring(0, 200))

  // 检查 Vue 是否挂载
  const hasContent = appContent.length > 100
  expect(hasContent).toBe(true)

  console.log('✓ 测试通过')
})
