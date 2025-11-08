/**
 * Playwright 全局 Teardown
 *
 * 在所有测试之后运行一次
 */

export default async function globalTeardown() {
  console.log('🧹 Global teardown running...')
  console.log('✅ Global teardown complete')
}
