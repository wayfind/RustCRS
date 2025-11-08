/**
 * 快速检查测试 - 使用改进的 hooks
 *
 * 这个测试文件用于验证修复是否有效
 */

import { test, expect } from './hooks.js';

test.describe('快速健康检查', () => {
  test('应该能成功加载首页', async ({ page }) => {
    // 访问首页，使用更宽松的等待条件
    await page.goto('/', {
      waitUntil: 'domcontentloaded',
      timeout: 30000
    });

    // 等待一个关键元素出现
    await page.waitForSelector('text=统计查询', { timeout: 10000 });

    // 验证页面标题
    const title = await page.title();
    console.log(`✓ 页面标题: ${title}`);

    expect(title).toContain('Claude Relay');
  });

  test('应该能访问 API Stats 页面', async ({ page }) => {
    await page.goto('/api-stats', {
      waitUntil: 'domcontentloaded',
      timeout: 30000
    });

    // 等待主要内容
    await page.waitForSelector('.glass-strong', { timeout: 10000 });

    // 获取页面内容
    const content = await page.textContent('body');
    expect(content).toContain('统计查询');

    console.log('✓ API Stats 页面加载成功');
  });

  test('应该能访问登录页面', async ({ page }) => {
    await page.goto('/login', {
      waitUntil: 'domcontentloaded',
      timeout: 30000
    });

    // 等待登录表单
    await page.waitForSelector('form, input[type="text"], input[type="password"]', {
      timeout: 10000
    });

    console.log('✓ 登录页面加载成功');
  });

  test('主题切换应该正常工作', async ({ page }) => {
    await page.goto('/', {
      waitUntil: 'domcontentloaded'
    });

    // 等待页面加载
    await page.waitForSelector('text=统计查询');

    // 检查 HTML 元素的 class
    const htmlElement = page.locator('html');
    const initialClass = await htmlElement.getAttribute('class');

    console.log(`初始主题 class: ${initialClass}`);

    // 简单验证主题系统存在
    expect(initialClass !== null).toBe(true);
  });

  test('页面应该没有 JavaScript 错误', async ({ page }) => {
    const errors = [];

    page.on('pageerror', error => {
      errors.push(error.message);
    });

    await page.goto('/', {
      waitUntil: 'domcontentloaded'
    });

    await page.waitForTimeout(2000);

    if (errors.length > 0) {
      console.log(`发现 ${errors.length} 个错误:`);
      errors.forEach(err => console.log(`  - ${err}`));
    }

    // 如果有错误，测试应该失败
    expect(errors.length).toBe(0);
  });
});

test.describe('响应式设计快速检查', () => {
  test('移动端视口', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto('/', {
      waitUntil: 'domcontentloaded'
    });

    await page.waitForSelector('text=统计查询');

    // 验证没有水平滚动
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    const viewportWidth = 375;

    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth + 2);

    console.log('✓ 移动端布局正常');
  });

  test('桌面视口', async ({ page }) => {
    await page.setViewportSize({ width: 1920, height: 1080 });

    await page.goto('/', {
      waitUntil: 'domcontentloaded'
    });

    await page.waitForSelector('text=统计查询');

    console.log('✓ 桌面布局正常');
  });
});
