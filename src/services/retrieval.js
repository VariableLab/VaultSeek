/**
 * Retrieval Service
 * 本地检索服务 - 整合 Embedding 和向量索引
 */

import embeddingService from './embedding.js';
import vectorIndexService from './vectorIndex.js';
import chunkerService from './chunker.js';

class RetrievalService {
  constructor() {
    this.initialized = false;
  }

  /**
   * 初始化服务
   */
  async init() {
    if (this.initialized) {
      return;
    }

    try {
      // 初始化 Embedding 服务
      await embeddingService.init();

      // 初始化向量索引服务
      await vectorIndexService.init();

      this.initialized = true;
      console.log('[Retrieval] 服务初始化完成');
    } catch (error) {
      console.error('[Retrieval] 初始化失败:', error);
      throw new Error('检索服务初始化失败：' + error.message);
    }
  }

  /**
   * 索引文档
   * @param {object} params - 文档对象
   * @param {string} params.id - 文档 ID
   * @param {string} params.text - 文档文本
   * @param {object} params.metadata - 元数据
   */
  async indexDocument({ id, text, metadata = {} }) {
    if (!this.initialized) {
      await this.init();
    }

    try {
      // 1. 文本分块
      const chunks = chunkerService.chunk(text);

      // 2. 为每个块生成向量
      const documents = [];
      for (let i = 0; i < chunks.length; i++) {
        const chunk = chunks[i];
        const embedding = await embeddingService.embed(chunk);

        documents.push({
          id: id + '-chunk-' + i,
          embedding,
          text: chunk,
          metadata: Object.assign({}, metadata, {
            chunkIndex: i,
            totalChunks: chunks.length,
          }),
        });
      }

      // 3. 批量添加到索引
      await vectorIndexService.addDocuments(documents);

      return {
        success: true,
        chunkCount: chunks.length,
      };
    } catch (error) {
      console.error('[Retrieval] 索引失败:', error);
      throw new Error('文档索引失败：' + error.message);
    }
  }

  /**
   * 检索相关文档
   * @param {string} query - 查询文本
   * @param {number} limit - 返回数量
   * @returns {Promise<Array>} - 搜索结果
   */
  async search(query, limit = 5) {
    if (!this.initialized) {
      await this.init();
    }

    try {
      // 1. 生成查询向量
      const queryEmbedding = await embeddingService.embed(query);

      // 2. 搜索相似文档
      const results = await vectorIndexService.search(queryEmbedding, limit);

      return results;
    } catch (error) {
      console.error('[Retrieval] 检索失败:', error);
      throw new Error('检索失败：' + error.message);
    }
  }

  /**
   * 删除文档
   */
  async deleteDocument(id) {
    if (!this.initialized) {
      await this.init();
    }

    return await vectorIndexService.deleteDocument(id);
  }

  /**
   * 清空索引
   */
  async clearIndex() {
    if (!this.initialized) {
      await this.init();
    }

    return await vectorIndexService.clear();
  }

  /**
   * 获取文档数量
   */
  async getDocumentCount() {
    if (!this.initialized) {
      await this.init();
    }

    return await vectorIndexService.count();
  }

  /**
   * 检查服务是否就绪
   */
  isReady() {
    return this.initialized && embeddingService.isReady() && vectorIndexService.isReady();
  }
}

// 单例模式
const retrievalService = new RetrievalService();
export default retrievalService;
