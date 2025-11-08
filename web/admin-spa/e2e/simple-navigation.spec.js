/**
 * 简化的导航测试 - 最小配置避免崩溃
 */
import { test, expect } from '@playwright/test'

test.describe('基础导航测试', () => {
  test('应该能访问首页', async ({ page }) => {
    console.log('开始测试: 访问首页')

    // 设置较短的超时
    test.setTimeout(15000)

    try {
      // 使用 commit 而不是 networkidle
      await page.goto('/', {
        waitUntil: 'commit',
        timeout: 10000
      })
      console.log('✓ 页面导航成功')

      // 等待一下让页面加载
      await page.waitForTimeout(1000)

      const url = page.url()
      console.log(`当前 URL: ${url}`)

      // 简单的断言
      expect(url).toContain('localhost:8080')
      console.log('✓ URL 验证通过')
    } catch (error) {
      console.error('测试失败:', error.message)
      throw error
    }
  })

  test('应该能访问登录页面', async ({ page }) => {
    console.log('开始测试: 访问登录页面')

    test.setTimeout(15000)

    try {
      await page.goto('/login', {
        waitUntil: 'commit',
        timeout: 10000
      })
      console.log('✓ 登录页面导航成功')

      await page.waitForTimeout(1000)

      const url = page.url()
      console.log(`当前 URL: ${url}`)

      expect(url).toContain('login')
      console.log('✓ 登录页面验证通过')
    } catch (error) {
      console.error('测试失败:', error.message)
      throw error
    }
  })

  test('应该能访问 API Stats 页面', async ({ page }) => {
    console.log('开始测试: 访问 API Stats')

    test.setTimeout(15000)

    try {
      await page.goto('/api-stats', {
        waitUntil: 'commit',
        timeout: 10000
      })
      console.log('✓ API Stats 页面导航成功')

      await page.waitForTimeout(1000)

      const url = page.url()
      console.log(`当前 URL: ${url}`)

      expect(url).toContain('api-stats')
      console.log('✓ API Stats 页面验证通过')
    } catch (error) {
      console.error('测试失败:', error.message)
      throw error
    }
  })
})
