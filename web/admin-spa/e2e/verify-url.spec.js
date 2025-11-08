import { test } from './hooks.js';

test('验证 admin-next 路径', async ({ page }) => {
  console.log('\n=== 测试 URL 访问 ===\n');
  
  // 明确访问 /admin-next/
  await page.goto('http://localhost:8080/admin-next/', { waitUntil: 'networkidle', timeout: 30000 });
  console.log('✓ 已导航到 /admin-next/');
  
  // 等待 Vue 加载
  await page.waitForTimeout(3000);
  
  const url = page.url();
  console.log(`当前 URL: ${url}`);
  
  const title = await page.title();
  console.log(`页面标题: "${title}"`);
  
  const appHTML = await page.locator('#app').innerHTML().catch(() => '');
  console.log(`#app innerHTML 长度: ${appHTML.length} 字符`);
  
  if (appHTML.length > 100) {
    console.log(`#app 前 200 字符: ${appHTML.substring(0, 200)}`);
  }
});
