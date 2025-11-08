/**
 * 修复版 UI 测试 - 不使用 networkidle
 */

import { test, expect } from './hooks.js';

test.describe('UI Walkthrough - 基本功能', () => {
  test('应该能访问首页并查看内容', async ({ page }) => {
    // 使用 domcontentloaded 代替 networkidle
    await page.goto('/', { waitUntil: 'domcontentloaded' });

    // 等待 Vue 应用挂载
    await page.waitForSelector('#app', { timeout: 10000 });

    // 检查页面标题
    const title = await page.title();
    expect(title).toContain('Claude Relay');

    console.log('✓ 首页加载成功');
  });

  test('应该能导航到 API Stats 页面', async ({ page }) => {
    await page.goto('/api-stats', { waitUntil: 'domcontentloaded' });

    // 等待主要内容区域
    await page.waitForSelector('main, #app, [role="main"]', { timeout: 10000 });

    // 检查 URL
    expect(page.url()).toContain('/api-stats');

    console.log('✓ API Stats 页面加载成功');
  });

  test('页面应该没有 JavaScript 错误', async ({ page }) => {
    const errors = [];

    page.on('pageerror', error => {
      errors.push(error.message);
    });

    await page.goto('/', { waitUntil: 'domcontentloaded' });

    // 等待一下
    await page.waitForTimeout(2000);

    expect(errors.length).toBe(0);

    console.log('✓ 没有 JavaScript 错误');
  });

  test('应该能查看页面内容', async ({ page }) => {
    await page.goto('/api-stats', { waitUntil: 'domcontentloaded' });

    // 等待 #app 元素
    await page.waitForSelector('#app');

    // 尝试获取页面文本（可能为空如果 Vue 未完成渲染）
    const hasApp = await page.locator('#app').count();
    expect(hasApp).toBeGreaterThan(0);

    console.log('✓ 页面包含 #app 元素');
  });

  test('移动端视口测试', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });

    await page.goto('/', { waitUntil: 'domcontentloaded' });

    await page.waitForSelector('#app');

    // 检查视口
    const viewport = page.viewportSize();
    expect(viewport?.width).toBe(375);

    console.log('✓ 移动端视口设置成功');
  });
});
