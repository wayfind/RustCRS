/**
 * Playwright 全局 Setup
 *
 * 在所有测试之前运行一次
 */

export default async function globalSetup() {
  console.log('🔧 Global setup running...');

  // 检查环境
  console.log(`   Base URL: ${process.env.BASE_URL || 'http://localhost:8080/admin-next'}`);
  console.log(`   CI: ${process.env.CI || 'false'}`);

  console.log('✅ Global setup complete');
}
