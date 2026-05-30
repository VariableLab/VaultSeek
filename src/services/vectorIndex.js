/**
 * Vector Index Service
 * 本地向量存储和检索（Electron 环境使用 SQLite，浏览器使用 localStorage）
 */
import pako from 'pako';

const STORAGE_KEY = 'vaultseek-vectors-compressed';
const META_KEY = 'vaultseek-vectors-meta';

const isElectron = () => typeof window !== 'undefined' && window.electronAPI;

class VectorIndexService {
  constructor() {
    this.vectors = new Map();
    this.initialized = false;
    this.vectorCount = 0;
    this.saveTimeout = null;
  }

  async init() {
    if (this.initialized) return;

    // Electron 环境从主进程加载
    if (isElectron()) {
      try {
        console.log('[VectorIndex] 从 Electron 加载向量...');
        const result = await window.electronAPI.getVectors();
        if (result.success && result.vectors && result.vectors.length > 0) {
          result.vectors.forEach((vec) => {
            // 解析 embedding（可能是字符串或数组）
            let embedding;
            if (typeof vec.embedding === 'string') {
              embedding = JSON.parse(vec.embedding);
            } else if (Array.isArray(vec.embedding)) {
              embedding = vec.embedding;
            } else {
              embedding = [];
            }

            this.vectors.set(vec.id, {
              embedding: new Float32Array(embedding),
              text: vec.chunk_text || '',
              metadata: {
                docId: vec.document_id,
                chunkIndex: vec.chunk_index,
                filename: vec.filename || ''
              },
              createdAt: Date.now(),
            });
          });
          this.vectorCount = this.vectors.size;
          console.log(`[VectorIndex] 从 SQLite 加载 ${this.vectorCount} 个向量`);
        } else {
          console.log('[VectorIndex] SQLite 中没有向量数据');
        }
      } catch (error) {
        console.error('[VectorIndex] 从 Electron 加载失败:', error);
      }
      this.initialized = true;
      return;
    }

    // 浏览器环境从 localStorage 加载
    try {
      const compressed = localStorage.getItem(STORAGE_KEY);
      if (compressed) {
        const decompressed = pako.inflate(compressed, { to: 'string' });
        const data = JSON.parse(decompressed);
        data.forEach((item) => {
          this.vectors.set(item.id, {
            embedding: new Float32Array(item.embedding),
            text: item.text,
            metadata: item.metadata,
            createdAt: item.createdAt,
          });
        });
        this.vectorCount = this.vectors.size;
        console.log(`[VectorIndex] 从 localStorage 加载 ${this.vectorCount} 个向量`);
      }
      this.initialized = true;
    } catch (error) {
      console.error('[VectorIndex] 初始化失败:', error);
      this.initialized = true;
      this.vectors.clear();
    }
  }

  /**
   * 延迟保存，避免频繁写入
   */
  scheduleSave() {
    if (this.saveTimeout) {
      clearTimeout(this.saveTimeout);
    }
    this.saveTimeout = setTimeout(() => {
      this.saveToStorage();
    }, 1000);
  }

  async saveToStorage() {
    // Electron 环境保存到主进程
    if (isElectron()) {
      try {
        const vectors = [];
        for (const [id, doc] of this.vectors.entries()) {
          vectors.push({
            id,
            documentId: doc.metadata?.docId || doc.metadata?.documentId || '',
            chunkIndex: doc.metadata?.chunkIndex || 0,
            chunkText: doc.text || '',
            embedding: Array.from(doc.embedding), // Float32Array -> 普通数组
          });
        }

        console.log(`[VectorIndex] 准备保存 ${vectors.length} 个向量到 SQLite`);

        if (vectors.length > 0) {
          const result = await window.electronAPI.saveVectors(vectors);
          this.vectorCount = this.vectors.size;
          console.log(`[VectorIndex] 保存到 SQLite 完成：${vectors.length} 个向量`);
          return result;
        } else {
          console.log('[VectorIndex] 没有向量需要保存');
          return { success: true, count: 0 };
        }
      } catch (error) {
        console.error('[VectorIndex] 保存到 Electron 失败:', error);
        return { success: false, error };
      }
    }

    // 浏览器环境保存到 localStorage
    try {
      const data = [];
      for (const [id, doc] of this.vectors.entries()) {
        data.push({
          id,
          embedding: Array.from(doc.embedding),
          text: doc.text,
          metadata: doc.metadata,
          createdAt: doc.createdAt,
        });
      }

      const jsonStr = JSON.stringify(data);
      const compressed = pako.gzip(jsonStr);
      const base64 = btoa(String.fromCharCode(...compressed));

      localStorage.setItem(STORAGE_KEY, base64);
      localStorage.setItem(META_KEY, JSON.stringify({
        count: data.length,
        savedAt: Date.now(),
        size: base64.length,
      }));

      this.vectorCount = this.vectors.size;
      console.log(`[VectorIndex] 保存到 localStorage: ${data.length} 个向量`);
    } catch (error) {
      console.error('[VectorIndex] 保存失败:', error);
    }
  }

  async addDocument({ id, embedding, text, metadata = {} }) {
    if (!this.initialized) await this.init();

    this.vectors.set(id, {
      embedding: new Float32Array(embedding),
      text,
      metadata,
      createdAt: Date.now(),
    });
  }

  async addDocuments(documents) {
    if (!this.initialized) await this.init();

    console.log(`[VectorIndex] 添加 ${documents.length} 个向量`);

    for (const doc of documents) {
      const { id, documentId, chunkIndex, chunkText, embedding } = doc;
      console.log(`  - ${id}: chunkText length=${chunkText?.length}, embedding length=${embedding?.length}`);

      this.vectors.set(id, {
        embedding: new Float32Array(embedding),
        text: chunkText,
        metadata: { docId: documentId, chunkIndex },
        createdAt: Date.now(),
      });
    }

    console.log(`[VectorIndex] 当前内存中有 ${this.vectors.size} 个向量`);

    // 批量保存
    await this.saveToStorage();
  }

  async search(queryEmbedding, limit = 5) {
    if (!this.initialized) await this.init();

    if (this.vectors.size === 0) {
      console.log('[VectorIndex] 搜索：向量库为空');
      return [];
    }

    const queryVector = new Float32Array(queryEmbedding);
    const results = [];

    for (const [id, doc] of this.vectors.entries()) {
      const similarity = this.cosineSimilarity(queryVector, doc.embedding);
      if (similarity > 0.3) {
        results.push({
          id,
          document: doc.text,
          metadata: doc.metadata,
          score: similarity,
        });
      }
    }

    results.sort((a, b) => b.score - a.score);
    console.log(`[VectorIndex] 搜索返回 ${results.length} 条结果`);
    return results.slice(0, limit);
  }

  cosineSimilarity(a, b) {
    let dotProduct = 0,
      normA = 0,
      normB = 0;
    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }
    if (normA === 0 || normB === 0) return 0;
    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  async deleteDocument(id) {
    if (!this.initialized) await this.init();

    // Electron 环境
    if (isElectron()) {
      await window.electronAPI.deleteVectors(id);
    }

    // 内存删除
    if (this.vectors.has(id)) {
      this.vectors.delete(id);
      await this.saveToStorage();
    }
  }

  async clear() {
    if (isElectron()) {
      await window.electronAPI.clearAllData();
    } else {
      localStorage.removeItem(STORAGE_KEY);
      localStorage.removeItem(META_KEY);
    }
    this.vectors.clear();
    this.vectorCount = 0;
    console.log('[VectorIndex] 已清空所有向量');
  }

  count() {
    return this.vectors.size;
  }

  getVectorCount() {
    return this.vectorCount;
  }

  isReady() {
    return this.initialized;
  }

  getStorageUsage() {
    if (isElectron()) {
      return { used: 0, limit: 50 * 1024 * 1024, percent: 0, count: this.vectorCount };
    }

    const meta = localStorage.getItem(META_KEY);
    const compressed = localStorage.getItem(STORAGE_KEY);

    if (!meta || !compressed) {
      return { used: 0, limit: 8 * 1024 * 1024, percent: 0, count: 0 };
    }

    const size = compressed.length * 2;
    return {
      used: size,
      limit: 8 * 1024 * 1024,
      percent: ((size / (8 * 1024 * 1024)) * 100).toFixed(1),
      count: this.vectorCount,
    };
  }
}

export default new VectorIndexService();
