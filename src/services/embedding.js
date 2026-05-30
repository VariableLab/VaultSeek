/**
 * Embedding Service
 * 使用 Ollama 调用 shaw/dmeta-embedding-zh 模型进行中文向量化
 */

const OLLAMA_BASE_URL = 'http://localhost:11434';
const MODEL_NAME = 'shaw/dmeta-embedding-zh:latest';

class EmbeddingService {
  constructor() {
    this.initialized = false;
    this.modelName = MODEL_NAME;
    this.dimension = 1024;
  }

  async init() {
    if (this.initialized) return;

    try {
      console.log('[Embedding] 连接 Ollama 服务...');

      // 检查 Ollama 是否可用
      const response = await fetch(`${OLLAMA_BASE_URL}/api/tags`);
      if (!response.ok) {
        throw new Error('Ollama 服务不可用');
      }

      const data = await response.json();
      const hasModel = data.models?.some(m => m.name.includes('shaw/dmeta-embedding'));

      if (!hasModel) {
        console.warn('[Embedding] 未找到 shaw/dmeta-embedding-zh 模型，尝试使用通用模型');
      }

      this.initialized = true;
      console.log('[Embedding] Ollama 连接成功，模型:', this.modelName);
    } catch (error) {
      console.error('[Embedding] 初始化失败:', error);
      throw new Error(`Ollama 服务不可用：${error.message}`);
    }
  }

  async embed(text) {
    if (!this.initialized) {
      await this.init();
    }

    try {
      const response = await fetch(`${OLLAMA_BASE_URL}/api/embeddings`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          model: this.modelName,
          prompt: text,
        }),
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(`Ollama API 错误：${error}`);
      }

      const data = await response.json();
      return data.embedding || [];
    } catch (error) {
      console.error('[Embedding] 向量化失败:', error);
      throw new Error(`向量化失败：${error.message}`);
    }
  }

  async embedBatch(texts) {
    const results = [];
    for (const text of texts) {
      const vector = await this.embed(text);
      results.push(vector);
    }
    return results;
  }

  getDimension() {
    return this.dimension;
  }

  isReady() {
    return this.initialized;
  }

  getStatus() {
    return {
      ready: this.initialized,
      model: this.modelName,
      ollamaUrl: OLLAMA_BASE_URL,
    };
  }

  reset() {
    this.initialized = false;
  }
}

export default new EmbeddingService();
