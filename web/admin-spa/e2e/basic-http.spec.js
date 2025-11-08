/**
 * 最基本的 HTTP 测试
 */

import { test, expect } from '@playwright/test';

test('基本 HTTP 请求', async ({ request }) => {
  console.log('发送 HTTP 请求到 http://localhost:3001/admin/...');

  const response = await request.get('http://localhost:3001/admin/');

  console.log('状态码:', response.status());
  console.log('状态文本:', response.statusText());

  const body = await response.text();
  console.log('响应体长度:', body.length);
  console.log('响应体 (前 500 字符):', body.substring(0, 500));

  expect(response.status()).toBe(200);
  expect(body).toContain('<div id="app"></div>');

  console.log('✓ HTTP 请求成功');
});

test('页面导航 - 不使用 hooks', async ({ page }) => {
  console.log('导航到 http://localhost:3001/admin/...');

  const response = await page.goto('http://localhost:3001/admin/', {
    waitUntil: 'commit',
    timeout: 10000
  });

  console.log('导航状态码:', response?.status());

  // 获取页面 HTML
  const html = await page.content();
  console.log('页面 HTML 长度:', html.length);
  console.log('是否包含 #app:', html.includes('id="app"'));

  // 检查 #app 是否存在（不等待）
  const appElement = await page.$('#app');
  console.log('#app 元素存在:', !!appElement);

  if (appElement) {
    const isVisible = await appElement.isVisible();
    console.log('#app 可见:', isVisible);
  }

  expect(html).toContain('id="app"');

  console.log('✓ 页面导航成功');
});
