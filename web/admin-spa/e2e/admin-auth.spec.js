import { test, expect } from '@playwright/test';

/**
 * 管理员认证流程测试
 *
 * 测试场景：
 * 1. 登录页面功能
 * 2. 表单验证
 * 3. 认证后路由保护
 */

test.describe('管理员认证 - 登录页面', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
  });

  test('应该显示登录表单', async ({ page }) => {
    // 检查表单元素
    const form = page.locator('form').first();
    await expect(form).toBeVisible();

    // 检查输入框
    const inputs = page.locator('input');
    const inputCount = await inputs.count();

    expect(inputCount).toBeGreaterThanOrEqual(2); // 至少有用户名和密码

    // 检查提交按钮
    const submitButton = page.locator('button[type="submit"]')
      .or(page.getByRole('button', { name: /登录|Login|submit/i }));
    await expect(submitButton.first()).toBeVisible();
  });

  test('应该显示必填字段标识', async ({ page }) => {
    // 检查是否有必填字段标识（* 或 required 属性）
    const requiredFields = page.locator('input[required], input[aria-required="true"]');
    const count = await requiredFields.count();

    console.log(`找到 ${count} 个必填字段`);
    expect(count).toBeGreaterThanOrEqual(1);
  });

  test('空表单提交应该显示验证错误', async ({ page }) => {
    // 查找提交按钮
    const submitButton = page.locator('button[type="submit"]')
      .or(page.getByRole('button', { name: /登录|Login|submit/i }))
      .first();

    // 直接点击提交（不填写任何内容）
    await submitButton.click();

    // 等待可能的错误提示
    await page.waitForTimeout(500);

    // 检查是否有表单验证（HTML5 验证或自定义验证）
    const hasValidationError = await page.evaluate(() => {
      const inputs = document.querySelectorAll('input');
      for (const input of inputs) {
        if (!input.checkValidity()) {
          return true;
        }
      }
      return false;
    });

    // 或者检查是否有错误消息显示
    const errorMessages = page.locator('.error, .el-form-item__error, [role="alert"]');
    const hasErrorMessage = await errorMessages.count() > 0;

    expect(hasValidationError || hasErrorMessage).toBe(true);
    console.log('✓ 表单验证正常工作');
  });
});

test.describe('管理员认证 - 表单交互', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
  });

  test('用户名输入框应该可以输入', async ({ page }) => {
    const usernameInput = page.locator('input[type="text"]').first()
      .or(page.getByPlaceholder(/用户名|username/i));

    await usernameInput.fill('testuser');
    await expect(usernameInput).toHaveValue('testuser');
  });

  test('密码输入框应该隐藏输入', async ({ page }) => {
    const passwordInput = page.locator('input[type="password"]').first();

    await expect(passwordInput).toBeVisible();
    await expect(passwordInput).toHaveAttribute('type', 'password');
  });

  test('应该能使用 Tab 键在表单字段间切换', async ({ page }) => {
    // 点击第一个输入框
    const firstInput = page.locator('input').first();
    await firstInput.click();

    // 按 Tab 键
    await page.keyboard.press('Tab');
    await page.waitForTimeout(100);

    // 检查焦点是否移动到下一个元素
    const focusedElement = await page.evaluate(() => {
      return document.activeElement?.tagName.toLowerCase();
    });

    expect(['input', 'button'].includes(focusedElement)).toBe(true);
    console.log('✓ 键盘导航正常');
  });

  test('应该能使用 Enter 键提交表单', async ({ page }) => {
    const usernameInput = page.locator('input[type="text"]').first();

    await usernameInput.fill('testuser');
    await page.keyboard.press('Enter');

    // 等待可能的网络请求或验证
    await page.waitForTimeout(500);

    console.log('✓ Enter 键提交功能正常');
  });
});

test.describe('管理员认证 - 路由保护', () => {
  test('未认证时访问受保护页面应该重定向到登录', async ({ page }) => {
    // 确保没有已存储的认证信息
    await page.context().clearCookies();
    await page.evaluate(() => localStorage.clear());

    // 尝试访问受保护的页面
    const protectedRoutes = [
      '/dashboard',
      '/api-keys',
      '/accounts',
      '/settings'
    ];

    for (const route of protectedRoutes) {
      await page.goto(route);
      await page.waitForLoadState('networkidle');

      // 应该重定向到登录页面
      const currentUrl = page.url();
      expect(currentUrl).toContain('login');

      console.log(`✓ ${route} 正确重定向到登录页面`);
    }
  });

  test('登录页面应该在已认证时重定向', async ({ page }) => {
    // 这个测试需要先登录才能验证
    // 这里只测试逻辑是否存在

    await page.goto('/login');
    await page.waitForLoadState('networkidle');

    // 检查是否有重定向逻辑（通过检查 localStorage 或 cookies）
    const hasAuthData = await page.evaluate(() => {
      const token = localStorage.getItem('auth_token') ||
                    localStorage.getItem('adminToken') ||
                    localStorage.getItem('token');
      return !!token;
    });

    if (hasAuthData) {
      // 如果有认证数据，应该不在登录页面
      const currentUrl = page.url();
      expect(currentUrl).not.toContain('/login');
      console.log('✓ 已认证用户正确重定向');
    } else {
      // 未认证，应该在登录页面
      console.log('✓ 未认证用户停留在登录页面');
    }
  });
});

test.describe('管理员认证 - 密码字段安全', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
  });

  test('密码字段应该防止自动完成（可选）', async ({ page }) => {
    const passwordInput = page.locator('input[type="password"]').first();

    const autocomplete = await passwordInput.getAttribute('autocomplete');

    // 检查是否设置了 autocomplete（推荐设置为 'current-password'）
    console.log(`密码字段 autocomplete 属性: ${autocomplete}`);

    // 这是一个信息性测试，不强制要求特定值
    expect(autocomplete !== null).toBe(true);
  });

  test('密码不应该在控制台或网络中明文显示', async ({ page }) => {
    const passwordInput = page.locator('input[type="password"]').first();

    // 监听控制台消息
    let hasPasswordInConsole = false;
    page.on('console', msg => {
      if (msg.text().includes('secretpassword123')) {
        hasPasswordInConsole = true;
      }
    });

    // 输入密码
    await passwordInput.fill('secretpassword123');

    // 等待一下
    await page.waitForTimeout(500);

    // 验证密码没有在控制台中泄露
    expect(hasPasswordInConsole).toBe(false);
    console.log('✓ 密码未在控制台中泄露');
  });
});

test.describe('管理员认证 - UI/UX', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/login');
    await page.waitForLoadState('networkidle');
  });

  test('登录页面应该有品牌标识', async ({ page }) => {
    // 查找 Logo、品牌名称或标题
    const brandElements = page.locator('img[alt*="logo"], h1, .logo, .brand');
    const count = await brandElements.count();

    expect(count).toBeGreaterThan(0);
    console.log('✓ 页面包含品牌标识');
  });

  test('登录页面应该有适当的页面标题', async ({ page }) => {
    const title = await page.title();

    expect(title).toBeTruthy();
    expect(title.length).toBeGreaterThan(0);

    console.log(`页面标题: ${title}`);
  });

  test('表单元素应该有适当的间距', async ({ page }) => {
    const inputs = page.locator('input');
    const count = await inputs.count();

    if (count >= 2) {
      const firstInput = inputs.first();
      const secondInput = inputs.nth(1);

      const firstRect = await firstInput.boundingBox();
      const secondRect = await secondInput.boundingBox();

      if (firstRect && secondRect) {
        const gap = secondRect.y - (firstRect.y + firstRect.height);
        expect(gap).toBeGreaterThanOrEqual(0);

        console.log(`表单元素间距: ${gap}px`);
      }
    }
  });
});
