/**
 * 立即与页面交互，不等待
 */

import { test, expect } from './hooks.js'

test('立即交互测试', async ({ page }) => {
  console.log('导航到 /api-stats...')
  await page.goto('/api-stats', { waitUntil: 'commit' })

  console.log('立即尝试获取标题...')
  try {
    const title = await page.title()
    console.log('✓ 页面标题:', title)
  } catch (e) {
    console.log('❌ 无法获取标题:', e.message)
  }

  console.log('等待 0.5 秒...')
  await page.waitForTimeout(500)

  console.log('尝试查找元素...')
  try {
    const body = await page.textContent('body')
    console.log('✓ Body 文本长度:', body?.length || 0)
  } catch (e) {
    console.log('❌ 无法获取 body:', e.message)
  }

  console.log('等待 1 秒...')
  await page.waitForTimeout(1000)

  console.log('再次尝试获取标题...')
  try {
    const title = await page.title()
    console.log('✓ 页面标题 (1秒后):', title)
  } catch (e) {
    console.log('❌ 无法获取标题 (1秒后):', e.message)
  }

  console.log('等待 3 秒...')
  await page.waitForTimeout(3000)

  console.log('最后尝试获取标题...')
  try {
    const title = await page.title()
    console.log('✓ 页面标题 (3秒后):', title)
  } catch (e) {
    console.log('❌ 无法获取标题 (3秒后):', e.message)
  }

  console.log('✓ 测试完成')
})

test('直接访问 API Stats，查找元素', async ({ page }) => {
  console.log('导航到 /api-stats...')
  await page.goto('/api-stats', { waitUntil: 'domcontentloaded', timeout: 10000 })

  console.log('等待 2 秒让 Vue 渲染...')
  await page.waitForTimeout(2000)

  console.log('尝试查找所有可能的元素...')

  const selectors = [
    '#app',
    'body',
    'main',
    '[role="main"]',
    '.container',
    '.api-stats',
    'h1',
    'h2',
    'div'
  ]

  for (const selector of selectors) {
    try {
      const count = await page.locator(selector).count()
      console.log(`  ${selector}: ${count} 个`)
      if (count > 0) {
        const isVisible = await page.locator(selector).first().isVisible()
        console.log(`    第一个是否可见: ${isVisible}`)
      }
    } catch (e) {
      console.log(`  ${selector}: 出错 - ${e.message}`)
    }
  }

  console.log('✓ 元素检查完成')
})
