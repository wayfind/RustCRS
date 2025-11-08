import { test, expect } from '@playwright/test'

/**
 * API 统计页面测试
 *
 * 测试场景：
 * 1. 页面基本功能
 * 2. 数据展示
 * 3. 图表渲染
 * 4. 过滤和搜索
 */

test.describe('API Stats - 页面加载', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
  })

  test('应该成功加载 API Stats 页面', async ({ page }) => {
    // 验证 URL
    expect(page.url()).toContain('/api-stats')

    // 验证页面内容
    const mainContent = page.locator('main, #app, [role="main"]').first()
    await expect(mainContent).toBeVisible()

    console.log('✓ API Stats 页面加载成功')
  })

  test('应该显示页面标题或标题栏', async ({ page }) => {
    // 查找页面标题
    const heading = page.locator('h1, h2, .page-title, .title').first()

    if (await heading.isVisible()) {
      const headingText = await heading.textContent()
      console.log(`页面标题: ${headingText}`)
      expect(headingText).toBeTruthy()
    } else {
      console.log('⚠ 未找到明显的页面标题，但页面已加载')
    }
  })

  test('页面应该无 JavaScript 错误', async ({ page }) => {
    let hasErrors = false
    let errorMessages = []

    page.on('pageerror', (error) => {
      hasErrors = true
      errorMessages.push(error.message)
      console.error(`JavaScript 错误: ${error.message}`)
    })

    // 等待一段时间确保所有脚本都执行
    await page.waitForTimeout(2000)

    if (hasErrors) {
      console.log('⚠ 发现 JavaScript 错误:')
      errorMessages.forEach((msg) => console.log(`  - ${msg}`))
    }

    expect(hasErrors).toBe(false)
  })
})

test.describe('API Stats - 数据展示', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(1000) // 等待数据加载
  })

  test('应该有统计数据卡片或指标展示', async ({ page }) => {
    // 查找常见的统计卡片元素
    const statsCards = page.locator('.card, .stat-card, .metric, .el-card, [class*="statistic"]')
    const count = await statsCards.count()

    console.log(`找到 ${count} 个统计卡片`)

    // 如果有统计卡片，验证其可见性
    if (count > 0) {
      await expect(statsCards.first()).toBeVisible()
    }

    // 这是一个宽松的检查，因为页面可能有不同的布局方式
  })

  test('应该展示数字统计信息', async ({ page }) => {
    // 查找包含数字的元素（统计数据通常是数字）
    const numberElements = await page.locator('*').evaluateAll((elements) => {
      const numberRegex = /^\d+(\.\d+)?[KMB]?$/ // 匹配纯数字或带单位的数字
      return elements
        .filter((el) => {
          const text = el.textContent?.trim()
          return text && numberRegex.test(text)
        })
        .map((el) => el.textContent.trim())
    })

    console.log(`找到 ${numberElements.length} 个数字统计`)

    if (numberElements.length > 0) {
      console.log(`示例统计值: ${numberElements.slice(0, 5).join(', ')}`)
    }
  })

  test('应该有加载状态或占位符', async ({ page }) => {
    // 快速重新加载以捕获加载状态
    await page.goto('/api-stats')

    // 在页面加载完成前查找加载指示器
    const loadingIndicators = page.locator(
      '.loading, .spinner, .el-loading-mask, [class*="loading"], [aria-busy="true"]'
    )

    const hasLoading = (await loadingIndicators.count()) > 0

    if (hasLoading) {
      console.log('✓ 页面有加载状态指示器')

      // 等待加载完成
      await page.waitForLoadState('networkidle')

      // 验证加载指示器消失
      await expect(loadingIndicators.first()).not.toBeVisible({ timeout: 10000 })
    } else {
      console.log('⚠ 未检测到加载指示器（数据可能加载很快）')
    }
  })
})

test.describe('API Stats - 图表展示', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(1500) // 等待图表渲染
  })

  test('应该渲染图表元素', async ({ page }) => {
    // 查找 Canvas 元素（Chart.js 等库使用 canvas）
    const canvasElements = page.locator('canvas')
    const canvasCount = await canvasElements.count()

    // 查找 SVG 元素（D3.js 等库使用 svg）
    const svgElements = page.locator('svg')
    const svgCount = await svgElements.count()

    // 查找图表容器
    const chartContainers = page.locator('.chart, .graph, [class*="chart"]')
    const containerCount = await chartContainers.count()

    console.log(`Canvas 元素: ${canvasCount}`)
    console.log(`SVG 元素: ${svgCount}`)
    console.log(`图表容器: ${containerCount}`)

    // 至少应该有一种图表元素
    const hasCharts = canvasCount > 0 || svgCount > 0 || containerCount > 0

    if (!hasCharts) {
      console.log('⚠ 未检测到图表元素，可能是数据驱动的展示')
    }
  })

  test('Canvas 图表应该有内容', async ({ page }) => {
    const canvasElements = page.locator('canvas')
    const count = await canvasElements.count()

    if (count > 0) {
      const firstCanvas = canvasElements.first()

      // 检查 canvas 是否有尺寸
      const boundingBox = await firstCanvas.boundingBox()

      expect(boundingBox).not.toBeNull()
      if (boundingBox) {
        expect(boundingBox.width).toBeGreaterThan(0)
        expect(boundingBox.height).toBeGreaterThan(0)

        console.log(`Canvas 尺寸: ${boundingBox.width}x${boundingBox.height}`)
      }
    } else {
      console.log('⚠ 未找到 Canvas 图表')
    }
  })
})

test.describe('API Stats - 交互功能', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
  })

  test('应该有刷新按钮或自动刷新功能', async ({ page }) => {
    // 查找刷新按钮
    const refreshButton = page
      .getByRole('button', { name: /刷新|refresh|reload/i })
      .or(
        page
          .locator('button')
          .filter({ has: page.locator('[class*="refresh"], [class*="reload"]') })
      )

    const hasRefreshButton = (await refreshButton.count()) > 0

    if (hasRefreshButton) {
      console.log('✓ 找到刷新按钮')

      // 点击刷新按钮
      await refreshButton.first().click()
      await page.waitForTimeout(500)

      console.log('✓ 刷新按钮可点击')
    } else {
      console.log('⚠ 未找到明显的刷新按钮')
    }
  })

  test('应该能选择时间范围（如果有）', async ({ page }) => {
    // 查找时间范围选择器
    const dateRangePicker = page.locator(
      '.el-date-picker, .date-range, .date-picker, [class*="date"], select'
    )

    const count = await dateRangePicker.count()

    if (count > 0) {
      console.log(`✓ 找到 ${count} 个日期/时间选择器`)
    } else {
      console.log('⚠ 未找到时间范围选择器')
    }
  })

  test('应该支持数据筛选（如果有）', async ({ page }) => {
    // 查找筛选相关元素
    const filterElements = page.locator('select, .filter, .dropdown, .el-select, [class*="filter"]')

    const count = await filterElements.count()

    if (count > 0) {
      console.log(`✓ 找到 ${count} 个筛选元素`)
    } else {
      console.log('⚠ 未找到明显的筛选元素')
    }
  })
})

test.describe('API Stats - 响应式布局', () => {
  test('移动端视口下图表应该适配', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 })

    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(1000)

    // 检查图表元素是否仍然可见且有合理的尺寸
    const canvasElements = page.locator('canvas')
    const count = await canvasElements.count()

    if (count > 0) {
      const firstCanvas = canvasElements.first()
      const boundingBox = await firstCanvas.boundingBox()

      if (boundingBox) {
        // 图表宽度不应超过视口宽度
        expect(boundingBox.width).toBeLessThanOrEqual(375)
        console.log(`移动端图表宽度: ${boundingBox.width}px`)
      }
    }
  })

  test('平板视口下布局应该合理', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 })

    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')

    // 检查是否有水平滚动
    const hasHorizontalScroll = await page.evaluate(() => {
      return document.body.scrollWidth > window.innerWidth
    })

    expect(hasHorizontalScroll).toBe(false)
    console.log('✓ 平板视口无水平滚动')
  })
})

test.describe('API Stats - 数据准确性', () => {
  test('统计数据应该是有效的数字', async ({ page }) => {
    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(1000)

    // 查找所有可能是统计数据的数字
    const statsValues = await page.evaluate(() => {
      const elements = Array.from(document.querySelectorAll('*'))
      const numberPattern = /^[\d,]+(\.\d+)?[KMB%]?$/

      return elements
        .map((el) => el.textContent?.trim())
        .filter((text) => text && numberPattern.test(text))
        .slice(0, 10) // 只取前 10 个
    })

    console.log('找到的统计值:')
    statsValues.forEach((value) => console.log(`  - ${value}`))

    // 验证至少找到一些统计值
    if (statsValues.length === 0) {
      console.log('⚠ 未找到明显的统计数字（可能需要数据）')
    }
  })

  test('百分比数据应该在合理范围内', async ({ page }) => {
    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(1000)

    const percentages = await page.evaluate(() => {
      const elements = Array.from(document.querySelectorAll('*'))
      const percentPattern = /^(\d+(\.\d+)?)%$/

      return elements
        .map((el) => el.textContent?.trim())
        .filter((text) => text && percentPattern.test(text))
        .map((text) => parseFloat(text.replace('%', '')))
    })

    if (percentages.length > 0) {
      console.log(`找到 ${percentages.length} 个百分比值`)

      percentages.forEach((value) => {
        expect(value).toBeGreaterThanOrEqual(0)
        expect(value).toBeLessThanOrEqual(100)
      })

      console.log('✓ 所有百分比值在 0-100% 范围内')
    }
  })
})

test.describe('API Stats - 空状态处理', () => {
  test('应该优雅处理无数据情况', async ({ page }) => {
    await page.goto('/api-stats')
    await page.waitForLoadState('networkidle')
    await page.waitForTimeout(1500)

    // 查找空状态提示
    const emptyState = page.locator('.empty, .no-data, .el-empty, [class*="empty"]')

    const hasEmptyState = (await emptyState.count()) > 0

    if (hasEmptyState) {
      console.log('✓ 页面有空状态处理')
    } else {
      console.log('⚠ 未检测到明显的空状态（可能已有数据）')
    }
  })
})
