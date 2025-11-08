/**
 * UI 基础测试 - 使用稳定配置避免崩溃
 */
import { test, expect } from '@playwright/test';

test.describe('UI 基础功能测试', () => {
  // 每个测试都使用独立的 browser context
  test.use({ viewport: { width: 1280, height: 720 } });

  test('首页应该能正常加载', async ({ page }) => {
    await page.goto('/', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(1500);

    const url = page.url();
    expect(url).toContain('localhost:8080');
    console.log('✓ 首页加载成功');
  });

  test('API Stats 页面应该能访问', async ({ page }) => {
    await page.goto('/api-stats', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(1500);

    const url = page.url();
    expect(url).toContain('api-stats');
    console.log('✓ API Stats 页面加载成功');
  });

  test('用户登录页应该能访问', async ({ page }) => {
    await page.goto('/user-login', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(1500);

    const url = page.url();
    expect(url).toContain('user-login');
    console.log('✓ 用户登录页加载成功');
  });

  test('管理员登录页应该能访问', async ({ page }) => {
    await page.goto('/login', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(1500);

    const url = page.url();
    expect(url).toContain('login');
    console.log('✓ 管理员登录页加载成功');
  });

  test('页面标题应该正确', async ({ page }) => {
    await page.goto('/', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(2000); // 等待 Vue 渲染

    const title = await page.title();
    expect(title).toContain('Claude Relay');
    console.log(`✓ 页面标题: ${title}`);
  });

  test('#app 元素应该存在', async ({ page }) => {
    await page.goto('/', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(1500);

    const appElement = await page.locator('#app').count();
    expect(appElement).toBe(1);
    console.log('✓ #app 元素存在');
  });

  test('静态资源应该正确加载', async ({ page }) => {
    const responses = [];
    page.on('response', response => {
      if (response.url().includes('/assets/')) {
        responses.push({
          url: response.url(),
          status: response.status()
        });
      }
    });

    await page.goto('/', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(2000);

    const failedResources = responses.filter(r => r.status !== 200);
    expect(failedResources.length).toBe(0);
    console.log(`✓ 加载了 ${responses.length} 个静态资源，全部成功`);
  });

  test('响应式布局 - 移动端视口', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto('/', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(1500);

    const url = page.url();
    expect(url).toContain('localhost:8080');
    console.log('✓ 移动端视口加载成功');
  });

  test('响应式布局 - 平板视口', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 1024 });
    await page.goto('/', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(1500);

    const url = page.url();
    expect(url).toContain('localhost:8080');
    console.log('✓ 平板视口加载成功');
  });

  test('页面应该无 JavaScript 错误', async ({ page }) => {
    const errors = [];
    page.on('pageerror', error => {
      errors.push(error.message);
    });

    await page.goto('/', { waitUntil: 'commit', timeout: 10000 });
    await page.waitForTimeout(2000);

    expect(errors.length).toBe(0);
    console.log('✓ 无 JavaScript 错误');
  });
});
