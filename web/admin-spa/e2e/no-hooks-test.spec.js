import { test, expect } from '@playwright/test';

test('访问页面 - 无 hooks', async ({ page }) => {
  console.log('\n=== 测试无 hooks 访问 ===\n');
  
  await page.goto('http://localhost:8080/admin-next/', { 
    waitUntil: 'domcontentloaded',
    timeout: 30000 
  });
  console.log('✓ 页面已加载');
  
  // 等待一下
  await page.waitForTimeout(2000);
  
  const title = await page.title();
  console.log(`页面标题: "${title}"`);
  
  const url = page.url();
  console.log(`当前 URL: ${url}`);
  
  // 简单断言
  expect(title).toContain('Claude Relay');
});
