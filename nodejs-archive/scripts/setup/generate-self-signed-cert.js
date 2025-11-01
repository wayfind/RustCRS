#!/usr/bin/env node
/**
 * 🔒 自签名 SSL 证书生成脚本 (Node.js 版本)
 * 用于开发和测试环境的 HTTPS 支持
 * 跨平台支持 (Windows/Linux/macOS)
 */

const { execSync } = require('child_process')
const fs = require('fs')
const path = require('path')
const readline = require('readline')

// 配置参数
const config = {
  certDir: path.join(process.cwd(), 'certs'),
  daysValid: 365,
  country: 'CN',
  state: 'Beijing',
  city: 'Beijing',
  org: 'Claude Relay Service',
  cn: 'localhost'
}

// 颜色输出（跨平台兼容）
const colors = {
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  reset: '\x1b[0m'
}

function log(message, color = 'reset') {
  console.log(`${colors[color]}${message}${colors.reset}`)
}

// 检查 openssl 是否可用
function checkOpenSSL() {
  try {
    execSync('openssl version', { stdio: 'pipe' })
    return true
  } catch (error) {
    return false
  }
}

// 创建 readline 接口
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
})

function question(query) {
  return new Promise((resolve) => rl.question(query, resolve))
}

async function main() {
  log('🔒 Claude Relay Service - 自签名证书生成工具', 'green')
  console.log('')

  // 检查 openssl
  if (!checkOpenSSL()) {
    log('❌ 错误: 未找到 openssl 命令', 'red')
    console.log('请安装 openssl:')
    console.log('  Ubuntu/Debian: sudo apt-get install openssl')
    console.log('  CentOS/RHEL: sudo yum install openssl')
    console.log('  macOS: brew install openssl')
    console.log('  Windows: 下载并安装 Win32/Win64 OpenSSL')
    console.log('  https://slproweb.com/products/Win32OpenSSL.html')
    process.exit(1)
  }

  // 创建证书目录
  if (!fs.existsSync(config.certDir)) {
    fs.mkdirSync(config.certDir, { recursive: true })
  }
  log(`📁 证书目录: ${config.certDir}`, 'green')

  // 自定义域名
  const inputCn = await question(`域名 (默认: ${config.cn}): `)
  if (inputCn.trim()) {
    config.cn = inputCn.trim()
  }

  // 自定义有效期
  const inputDays = await question(`证书有效期（天数，默认: ${config.daysValid}）: `)
  if (inputDays.trim()) {
    const days = parseInt(inputDays.trim())
    if (!isNaN(days) && days > 0) {
      config.daysValid = days
    }
  }

  rl.close()

  const certFile = path.join(config.certDir, 'cert.pem')
  const keyFile = path.join(config.certDir, 'key.pem')

  console.log('')
  log('⚙️  生成配置:', 'yellow')
  console.log(`   域名: ${config.cn}`)
  console.log(`   有效期: ${config.daysValid} 天`)
  console.log(`   证书路径: ${certFile}`)
  console.log(`   私钥路径: ${keyFile}`)
  console.log('')

  // 生成私钥和自签名证书
  log('🔐 生成私钥和证书...', 'green')

  const opensslCmd = [
    'openssl',
    'req',
    '-x509',
    '-nodes',
    `-days ${config.daysValid}`,
    '-newkey rsa:2048',
    `-keyout "${keyFile}"`,
    `-out "${certFile}"`,
    `-subj "/C=${config.country}/ST=${config.state}/L=${config.city}/O=${config.org}/CN=${config.cn}"`,
    `-addext "subjectAltName=DNS:${config.cn},DNS:*.${config.cn},IP:127.0.0.1,IP:0.0.0.0"`
  ].join(' ')

  try {
    execSync(opensslCmd, { stdio: 'pipe' })

    // 设置文件权限 (仅 Unix-like 系统)
    if (process.platform !== 'win32') {
      fs.chmodSync(keyFile, 0o600)
      fs.chmodSync(certFile, 0o644)
    }

    console.log('')
    log('✅ 证书生成成功！', 'green')
    console.log('')

    // 显示证书信息
    log('📋 证书信息:', 'yellow')
    try {
      const certInfo = execSync(`openssl x509 -in "${certFile}" -noout -text`, {
        encoding: 'utf8'
      })
      const relevantLines = certInfo
        .split('\n')
        .filter(
          (line) =>
            line.includes('Subject:') ||
            line.includes('Not Before') ||
            line.includes('Not After') ||
            line.includes('DNS:')
        )
      console.log(relevantLines.join('\n'))
    } catch (error) {
      console.log('   (无法获取证书详细信息)')
    }

    console.log('')
    log('📝 使用方法:', 'yellow')
    console.log('1. 更新 .env 文件:')
    console.log('   HTTPS_ENABLED=true')
    console.log('   HTTPS_PORT=3443')
    console.log(`   HTTPS_CERT_PATH=${certFile}`)
    console.log(`   HTTPS_KEY_PATH=${keyFile}`)
    console.log('   HTTPS_REDIRECT_HTTP=true')
    console.log('')
    console.log('2. 启动服务:')
    console.log('   npm start')
    console.log('')
    console.log('3. 访问 HTTPS 服务:')
    console.log(`   https://${config.cn}:3443`)
    console.log('')
    log('⚠️  安全提示:', 'yellow')
    console.log('   - 自签名证书仅用于开发/测试环境')
    console.log('   - 浏览器会显示安全警告（正常现象）')
    console.log('   - 生产环境请使用 Let\'s Encrypt 或商业 CA 证书')
    console.log(`   - 不要将私钥文件 (${keyFile}) 提交到版本控制`)
    console.log('')
  } catch (error) {
    log('❌ 证书生成失败:', 'red')
    console.error(error.message)
    process.exit(1)
  }
}

main().catch((error) => {
  log('❌ 发生错误:', 'red')
  console.error(error)
  process.exit(1)
})
