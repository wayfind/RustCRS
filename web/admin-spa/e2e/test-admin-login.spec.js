/**
 * 测试直接访问管理登录页
 */

import { test, expect } from './hooks.js'

test('访问管理登录页', async ({ page }) => {
  const logs = []

  page.on('console', (msg) => {
    const text = msg.text()
    logs.push({ type: msg.type(), text })
    console.log(`[${msg.type()}]`, text)
  })

  console.log('导航到 /admin-login...')
  await page.goto('/admin-login', { waitUntil: 'domcontentloaded', timeout: 15000 })

  console.log('等待 2 秒...')
  await page.waitForTimeout(2000)

  console.log('尝试查找登录表单元素...')

  try {
    // 检查是否有表单
    const formCount = await page.locator('form').count()
    console.log(`表单数量: ${formCount}`)
  } catch (e) {
    console.log('无法查找表单:', e.message)
  }

  try {
    // 检查是否有输入框
    const inputCount = await page.locator('input').count()
    console.log(`输入框数量: ${inputCount}`)
  } catch (e) {
    console.log('无法查找输入框:', e.message)
  }

  try {
    // 获取页面文本
    const text = await page.textContent('body')
    console.log(`Body 文本长度: ${text?.length || 0}`)
    if (text && text.length > 0) {
      console.log(`前 200 字符: ${text.substring(0, 200)}`)
    }
  } catch (e) {
    console.log('无法获取页面文本:', e.message)
  }

  console.log(`\n控制台消息总数: ${logs.length}`)

  console.log('✓ 测试完成')
})
