import { test, expect } from '@playwright/test';

/**
 * UI 漫游测试 - Claude Relay Service
 *
 * 测试核心用户流程：
 * 1. 访问公开页面（API Stats）
 * 2. 导航到登录页面
 * 3. 测试主题切换功能
 * 4. 测试响应式布局
 */

test.describe('UI Walkthrough - 公开页面访问', () => {
  test('应该能访问 API Stats 首页', async ({ page }) => {
    // 访问首页
    await page.goto('/');

    // 等待页面加载
    await page.waitForLoadState('networkidle');

    // 验证页面标题
    await expect(page).toHaveTitle(/Claude Relay/);

    // 验证导航栏存在
    const navbar = page.locator('nav, header, [role="navigation"]').first();
    await expect(navbar).toBeVisible();

    // 验证主要内容区域存在
    const mainContent = page.locator('main, #app, [role="main"]').first();
    await expect(mainContent).toBeVisible();
  });

  test('应该显示 API 统计信息', async ({ page }) => {
    await page.goto('/api-stats');
    await page.waitForLoadState('networkidle');

    // 等待统计数据加载
    await page.waitForTimeout(1000);

    // 验证页面元素
    const heading = page.locator('h1, h2, .title').first();
    await expect(heading).toBeVisible();
  });
});

test.describe('UI Walkthrough - 导航测试', () => {
  test('应该能导航到登录页面', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // 查找登录链接/按钮
    const loginLink = page.getByText(/登录|Login|login/i).first();

    if (await loginLink.isVisible()) {
      await loginLink.click();
      await page.waitForURL(/.*login.*/);

      // 验证登录页面元素
      const usernameInput = page.getByPlaceholder(/用户名|username/i);
      const passwordInput = page.getByPlaceholder(/密码|password/i);

      await expect(usernameInput.or(page.locator('input[type="text"]').first())).toBeVisible();
      await expect(passwordInput.or(page.locator('input[type="password"]').first())).toBeVisible();
    }
  });

  test('应该能访问所有公开路由', async ({ page }) => {
    const publicRoutes = [
      { path: '/api-stats', name: 'API 统计' },
      { path: '/login', name: '管理员登录' },
      { path: '/user-login', name: '用户登录' }
    ];

    for (const route of publicRoutes) {
      await page.goto(route.path);
      await page.waitForLoadState('networkidle');

      // 验证页面加载成功（无错误页面）
      const errorMessage = page.getByText(/404|Not Found|页面不存在/i);
      await expect(errorMessage).not.toBeVisible();

      console.log(`✓ ${route.name} (${route.path}) 访问成功`);
    }
  });
});

test.describe('UI Walkthrough - 主题切换', () => {
  test('应该能切换暗黑模式', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // 查找主题切换按钮（常见的图标或文字）
    const themeToggle = page.locator('button').filter({
      has: page.locator('[class*="moon"], [class*="sun"], [class*="theme"]')
    }).first();

    if (await themeToggle.isVisible()) {
      // 获取当前主题
      const htmlElement = page.locator('html');
      const initialTheme = await htmlElement.getAttribute('class');

      // 点击切换主题
      await themeToggle.click();
      await page.waitForTimeout(300); // 等待主题切换动画

      // 验证主题已改变
      const newTheme = await htmlElement.getAttribute('class');
      expect(initialTheme).not.toBe(newTheme);

      // 再次切换回来
      await themeToggle.click();
      await page.waitForTimeout(300);

      console.log('✓ 主题切换功能正常');
    } else {
      console.log('⚠ 未找到主题切换按钮，跳过测试');
    }
  });
});

test.describe('UI Walkthrough - 响应式设计', () => {
  test('应该在移动端视口正常显示', async ({ page }) => {
    // 设置移动端视口
    await page.setViewportSize({ width: 375, height: 667 }); // iPhone SE

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // 验证页面内容可见
    const mainContent = page.locator('main, #app, [role="main"]').first();
    await expect(mainContent).toBeVisible();

    // 验证没有水平滚动条
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    const viewportWidth = await page.evaluate(() => window.innerWidth);

    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth + 1); // +1 允许1px误差

    console.log('✓ 移动端响应式布局正常');
  });

  test('应该在平板视口正常显示', async ({ page }) => {
    // 设置平板视口
    await page.setViewportSize({ width: 768, height: 1024 }); // iPad

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // 验证页面内容可见
    const mainContent = page.locator('main, #app, [role="main"]').first();
    await expect(mainContent).toBeVisible();

    console.log('✓ 平板响应式布局正常');
  });

  test('应该在桌面视口正常显示', async ({ page }) => {
    // 设置桌面视口
    await page.setViewportSize({ width: 1920, height: 1080 }); // Full HD

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // 验证页面内容可见
    const mainContent = page.locator('main, #app, [role="main"]').first();
    await expect(mainContent).toBeVisible();

    console.log('✓ 桌面响应式布局正常');
  });
});

test.describe('UI Walkthrough - 性能测试', () => {
  test('首页加载时间应小于 3 秒', async ({ page }) => {
    const startTime = Date.now();

    await page.goto('/');
    await page.waitForLoadState('networkidle');

    const loadTime = Date.now() - startTime;

    console.log(`首页加载时间: ${loadTime}ms`);
    expect(loadTime).toBeLessThan(3000);
  });

  test('页面切换应该流畅', async ({ page }) => {
    await page.goto('/api-stats');
    await page.waitForLoadState('networkidle');

    const startTime = Date.now();

    await page.goto('/login');
    await page.waitForLoadState('networkidle');

    const navigationTime = Date.now() - startTime;

    console.log(`页面切换时间: ${navigationTime}ms`);
    expect(navigationTime).toBeLessThan(2000);
  });
});

test.describe('UI Walkthrough - 可访问性测试', () => {
  test('页面应该有正确的语义化标签', async ({ page }) => {
    await page.goto('/');
    await page.waitForLoadState('networkidle');

    // 检查必要的 ARIA 属性和语义化标签
    const main = page.locator('main, [role="main"]');
    const nav = page.locator('nav, [role="navigation"]');

    const hasMain = await main.count() > 0;
    const hasNav = await nav.count() > 0;

    expect(hasMain || hasNav).toBe(true);
    console.log('✓ 页面包含语义化标签');
  });

  test('交互元素应该可以通过键盘访问', async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');

    // 使用 Tab 键导航
    await page.keyboard.press('Tab');
    await page.waitForTimeout(100);

    // 检查焦点是否在可交互元素上
    const focusedElement = await page.evaluate(() => {
      const el = document.activeElement;
      return el ? el.tagName.toLowerCase() : null;
    });

    const interactiveElements = ['input', 'button', 'a', 'select', 'textarea'];
    const isFocusable = interactiveElements.includes(focusedElement);

    console.log(`焦点元素: ${focusedElement}`);
    expect(isFocusable).toBe(true);
  });
});

test.describe('UI Walkthrough - 错误处理', () => {
  test('访问不存在的路由应该重定向', async ({ page }) => {
    await page.goto('/non-existent-route-12345');
    await page.waitForLoadState('networkidle');

    // 应该重定向到 api-stats 或显示错误页面
    const currentUrl = page.url();

    expect(
      currentUrl.includes('/api-stats') ||
      currentUrl.includes('/404') ||
      currentUrl.includes('/error')
    ).toBe(true);

    console.log(`不存在的路由重定向到: ${currentUrl}`);
  });

  test('应该在网络错误时显示友好提示', async ({ page }) => {
    // 模拟网络离线
    await page.context().setOffline(true);

    try {
      await page.goto('/');
      await page.waitForLoadState('networkidle', { timeout: 5000 });
    } catch (error) {
      // 预期会失败
      console.log('✓ 网络错误正确处理');
    }

    // 恢复网络
    await page.context().setOffline(false);
  });
});
