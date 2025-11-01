#!/bin/bash
# 🔒 自签名 SSL 证书生成脚本
# 用于开发和测试环境的 HTTPS 支持

set -e

# 配置参数
CERT_DIR="./certs"
DAYS_VALID=365
COUNTRY="CN"
STATE="Beijing"
CITY="Beijing"
ORG="Claude Relay Service"
CN="localhost"

# 颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}🔒 Claude Relay Service - 自签名证书生成工具${NC}"
echo ""

# 检查 openssl
if ! command -v openssl &> /dev/null; then
    echo -e "${RED}❌ 错误: 未找到 openssl 命令${NC}"
    echo "请安装 openssl:"
    echo "  Ubuntu/Debian: sudo apt-get install openssl"
    echo "  CentOS/RHEL: sudo yum install openssl"
    echo "  macOS: brew install openssl"
    exit 1
fi

# 创建证书目录
mkdir -p "$CERT_DIR"
echo -e "${GREEN}📁 证书目录: $CERT_DIR${NC}"

# 自定义域名（可选）
read -p "域名 (默认: localhost): " input_cn
if [ ! -z "$input_cn" ]; then
    CN="$input_cn"
fi

# 自定义有效期（可选）
read -p "证书有效期（天数，默认: 365）: " input_days
if [ ! -z "$input_days" ]; then
    DAYS_VALID="$input_days"
fi

CERT_FILE="$CERT_DIR/cert.pem"
KEY_FILE="$CERT_DIR/key.pem"

echo ""
echo -e "${YELLOW}⚙️  生成配置:${NC}"
echo "   域名: $CN"
echo "   有效期: $DAYS_VALID 天"
echo "   证书路径: $CERT_FILE"
echo "   私钥路径: $KEY_FILE"
echo ""

# 生成私钥和自签名证书
echo -e "${GREEN}🔐 生成私钥和证书...${NC}"
openssl req -x509 -nodes -days $DAYS_VALID -newkey rsa:2048 \
    -keyout "$KEY_FILE" \
    -out "$CERT_FILE" \
    -subj "/C=$COUNTRY/ST=$STATE/L=$CITY/O=$ORG/CN=$CN" \
    -addext "subjectAltName=DNS:$CN,DNS:*.${CN},IP:127.0.0.1,IP:0.0.0.0"

# 设置文件权限
chmod 600 "$KEY_FILE"
chmod 644 "$CERT_FILE"

echo ""
echo -e "${GREEN}✅ 证书生成成功！${NC}"
echo ""
echo -e "${YELLOW}📋 证书信息:${NC}"
openssl x509 -in "$CERT_FILE" -noout -text | grep -E "Subject:|Not Before|Not After|DNS:"
echo ""
echo -e "${YELLOW}📝 使用方法:${NC}"
echo "1. 更新 .env 文件:"
echo "   HTTPS_ENABLED=true"
echo "   HTTPS_PORT=3443"
echo "   HTTPS_CERT_PATH=$(pwd)/$CERT_FILE"
echo "   HTTPS_KEY_PATH=$(pwd)/$KEY_FILE"
echo "   HTTPS_REDIRECT_HTTP=true"
echo ""
echo "2. 启动服务:"
echo "   npm start"
echo ""
echo "3. 访问 HTTPS 服务:"
echo "   https://$CN:3443"
echo ""
echo -e "${YELLOW}⚠️  安全提示:${NC}"
echo "   - 自签名证书仅用于开发/测试环境"
echo "   - 浏览器会显示安全警告（正常现象）"
echo "   - 生产环境请使用 Let's Encrypt 或商业 CA 证书"
echo "   - 不要将私钥文件 ($KEY_FILE) 提交到版本控制"
echo ""
