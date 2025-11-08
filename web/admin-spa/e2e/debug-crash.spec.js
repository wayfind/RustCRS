import { test, expect } from '@playwright/test';

/**
 * 调试页面崩溃问题
 */

test('捕获页面崩溃错误', async ({ page }) => {
  let crashErrors = [];
  let consoleMessages = [];
  let pageErrors = [];

  // 监听页面崩溃
  page.on('crash', () => {
    crashErrors.push('Page crashed!');
    console.log('❌ Page crashed!');
  });

  // 监听控制台消息
  page.on('console', msg => {
    const text = `[${msg.type()}] ${msg.text()}`;
    consoleMessages.push(text);
    console.log(text);
  });

  // 监听页面错误
  page.on('pageerror', error => {
    const errorText = `Page Error: ${error.message}\nStack: ${error.stack}`;
    pageErrors.push(errorText);
    console.error(errorText);
  });

  // 监听请求失败
  page.on('requestfailed', request => {
    console.log(`Request failed: ${request.url()} - ${request.failure().errorText}`);
  });

  try {
    console.log('尝试访问首页...');
    await page.goto('http://localhost:3001/admin/', {
      waitUntil: 'domcontentloaded',
      timeout: 30000
    });

    console.log('✓ 页面导航成功');

    // 等待一下看是否有延迟错误
    await page.waitForTimeout(2000);

    console.log('✓ 等待 2 秒后无崩溃');

    // 尝试获取页面标题
    const title = await page.title();
    console.log(`页面标题: ${title}`);

  } catch (error) {
    console.error(`测试失败: ${error.message}`);
    console.error(`Crash errors: ${JSON.stringify(crashErrors)}`);
    console.error(`Console messages: ${JSON.stringify(consoleMessages.slice(0, 10))}`);
    console.error(`Page errors: ${JSON.stringify(pageErrors)}`);

    throw error;
  }
});

test('测试简化的页面加载', async ({ page }) => {
  console.log('使用简化方法访问页面...');

  try {
    const response = await page.goto('http://localhost:3001/admin/', {
      waitUntil: 'commit',  // 更宽松的等待条件
      timeout: 30000
    });

    console.log(`Response status: ${response.status()}`);
    console.log(`Response URL: ${response.url()}`);

    // 尝试获取 HTML 内容
    const content = await page.content();
    console.log(`HTML length: ${content.length}`);
    console.log(`HTML preview: ${content.substring(0, 200)}`);

  } catch (error) {
    console.error(`简化测试失败: ${error.message}`);
    throw error;
  }
});
