/**
 * 诊断测试 - 查看页面实际渲染的内容
 */

import { test, expect } from './hooks.js'

test('诊断：查看页面内容', async ({ page }) => {
  console.log('📋 开始诊断...')

  await page.goto('/', {
    waitUntil: 'domcontentloaded',
    timeout: 30000
  })

  console.log('✓ 页面已导航')

  // 等待一下让 JavaScript 执行
  await page.waitForTimeout(3000)

  // 获取页面 HTML
  const html = await page.content()
  console.log(`HTML 长度: ${html.length} 字符`)

  // 获取 body 内容
  const bodyText = await page.textContent('body')
  console.log('Body 文本内容:')
  console.log(bodyText.substring(0, 500))

  // 获取所有可见元素
  const elements = await page.evaluate(() => {
    const all = Array.from(document.querySelectorAll('*'))
    const visible = all.filter((el) => {
      const style = window.getComputedStyle(el)
      return style.display !== 'none' && style.visibility !== 'hidden'
    })
    return visible
      .map((el) => ({
        tag: el.tagName,
        id: el.id,
        classes: Array.from(el.classList),
        text: el.textContent?.substring(0, 50)
      }))
      .slice(0, 20)
  })

  console.log('\n前 20 个可见元素:')
  elements.forEach((el, i) => {
    console.log(
      `${i + 1}. <${el.tag}> ${el.id ? `#${el.id}` : ''} ${el.classes.join('.')} - "${el.text}"`
    )
  })

  // 检查 Vue 是否挂载
  const hasVueApp = await page.evaluate(() => {
    return (
      !!document.querySelector('[data-v-app]') ||
      !!window.__VUE__ ||
      !!document.querySelector('#app')
    )
  })

  console.log(`\nVue 应用挂载: ${hasVueApp}`)

  // 检查路由
  const currentUrl = page.url()
  console.log(`当前 URL: ${currentUrl}`)

  // 检查是否有错误
  const errors = await page.evaluate(() => {
    return window.__playwright_errors || []
  })

  if (errors.length > 0) {
    console.log('\n发现错误:')
    errors.forEach((err) => console.log(`  - ${err}`))
  }

  // 截图
  await page.screenshot({ path: 'playwright-report/diagnose-screenshot.png', fullPage: true })
  console.log('\n📸 已保存截图到 playwright-report/diagnose-screenshot.png')
})

test('诊断：检查 API 端点', async ({ page }) => {
  console.log('\n🔍 检查 API 端点...')

  const apiEndpoints = ['/webapi/health', '/webapi/oem/settings', '/health']

  for (const endpoint of apiEndpoints) {
    try {
      const response = await page.request.get(`http://localhost:8080${endpoint}`)
      console.log(`${endpoint}: ${response.status()} ${response.statusText()}`)

      if (response.ok()) {
        const body = await response.text()
        console.log(`  Response: ${body.substring(0, 100)}`)
      }
    } catch (error) {
      console.log(`${endpoint}: ❌ ${error.message}`)
    }
  }
})

test('诊断：检查前端服务器', async ({ page }) => {
  console.log('\n🌐 检查前端服务器...')

  try {
    const response = await page.request.get('http://localhost:3001/admin/')
    console.log(`前端服务器: ${response.status()} ${response.statusText()}`)

    const headers = response.headers()
    console.log('Response Headers:')
    for (const [key, value] of Object.entries(headers)) {
      console.log(`  ${key}: ${value}`)
    }
  } catch (error) {
    console.log(`前端服务器: ❌ ${error.message}`)
  }
})
