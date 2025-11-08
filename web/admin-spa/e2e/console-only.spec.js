/**
 * 只捕获控制台输出，不执行复杂操作
 */

import { test, expect } from './hooks.js'

test('捕获控制台输出', async ({ page }) => {
  const logs = { console: [], errors: [], failed: [] }

  page.on('console', (msg) => {
    const text = msg.text()
    logs.console.push({ type: msg.type(), text })
    console.log(`[${msg.type()}]`, text)
  })

  page.on('pageerror', (error) => {
    logs.errors.push(error.message)
    console.log('❌', error.message)
  })

  page.on('requestfailed', (request) => {
    const url = request.url()
    logs.failed.push(url)
    console.log('⚠️ ', url)
  })

  console.log('导航到首页...')
  const response = await page.goto('/', { waitUntil: 'commit', timeout: 10000 })
  console.log('响应状态:', response?.status())

  console.log('等待 5 秒...')
  try {
    await page.waitForTimeout(5000)
    console.log('等待完成')
  } catch (error) {
    console.log('等待期间出错:', error.message)
  }

  console.log('\n=== 总结 ===')
  console.log('控制台消息:', logs.console.length)
  console.log('页面错误:', logs.errors.length)
  console.log('失败请求:', logs.failed.length)

  // 尝试获取页面状态（可能会崩溃）
  try {
    const title = await page.title()
    console.log('页面标题:', title)
  } catch (e) {
    console.log('无法获取标题:', e.message)
  }

  console.log('✓ 测试完成')
})
