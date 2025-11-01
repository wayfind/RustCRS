#!/bin/bash
# 从 .env 文件加载环境变量到当前 shell

# 检查 .env 文件是否存在
if [ ! -f ".env" ]; then
    echo "错误: .env 文件不存在"
    exit 1
fi

# 读取 .env 文件并显式 export 每个变量
# 这样可以确保变量被传递给子进程（如 cargo）
while IFS='=' read -r key value; do
    # 跳过注释和空行
    [[ $key =~ ^#.*$ ]] && continue
    [[ -z $key ]] && continue

    # 移除值两边的引号（如果有）
    value="${value%\"}"
    value="${value#\"}"
    value="${value%\'}"
    value="${value#\'}"

    # 显式 export
    export "$key=$value"
done < .env

echo "✅ 环境变量已从 .env 加载"
