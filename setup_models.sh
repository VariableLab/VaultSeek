#!/bin/bash

# VaultSeek AI 模型下载脚本
# 模型：BAAI/bge-small-zh-v1.5 (ONNX 版)

RESOURCE_DIR="src-tauri/resources"
MODEL_URL="https://huggingface.co/Xenova/bge-small-zh-v1.5/resolve/main/onnx/model.onnx"
TOKENIZER_URL="https://huggingface.co/Xenova/bge-small-zh-v1.5/resolve/main/tokenizer.json"

echo "🚀 开始配置 VaultSeek AI 引擎资源..."

# 1. 创建目录
mkdir -p $RESOURCE_DIR

# 2. 下载模型文件 (model.onnx)
if [ ! -f "$RESOURCE_DIR/model.onnx" ]; then
    echo "📥 正在下载语义模型 (约 45MB)..."
    curl -L $MODEL_URL -o "$RESOURCE_DIR/model.onnx"
else
    echo "✅ 模型文件已存在，跳过下载。"
fi

# 3. 下载分词器文件 (tokenizer.json)
if [ ! -f "$RESOURCE_DIR/tokenizer.json" ]; then
    echo "📥 正在下载分词器配置..."
    curl -L $TOKENIZER_URL -o "$RESOURCE_DIR/tokenizer.json"
else
    echo "✅ 分词器已存在，跳过下载。"
fi

echo "✨ 所有 AI 资源配置完成！路径: $RESOURCE_DIR"
