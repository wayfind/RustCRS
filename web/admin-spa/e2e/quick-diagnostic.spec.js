import { test } from './hooks.js'

test('快速诊断页面加载', async ({ page }) => {
  console.log('\n=== 开始诊断 ===\n')

  // 导航到首页
  await page.goto('/', { waitUntil: 'networkidle', timeout: 30000 })
  console.log('✓ 页面已加载')

  // 等待 Vue 加载
  await page.waitForTimeout(3000)

  // 检查页面内容
  const html = await page.content()
  console.log(`HTML 长度: ${html.length} 字符`)

  const title = await page.title()
  console.log(`页面标题: "${title}"`)

  const url = page.url()
  console.log(`当前 URL: ${url}`)

  // 检查 #app 元素
  const appHTML = await page
    .locator('#app')
    .innerHTML()
    .catch(() => '')
  console.log(`#app innerHTML 长度: ${appHTML.length} 字符`)

  if (appHTML.length > 0) {
    console.log(`#app 前 200 字符: ${appHTML.substring(0, 200)}`)
  }

  // 检查是否有可见元素
  const visibleElements = await page.locator('*:visible').count()
  console.log(`可见元素数量: ${visibleElements}`)

  // 截图
  await page.screenshot({ path: 'playwright-report/diagnostic.png', fullPage: true })
  console.log('✓ 截图已保存')
})
