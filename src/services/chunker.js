/**
 * Text Chunking Service
 * 将长文本分割成适合向量化的块
 */

class ChunkerService {
  constructor(options = {}) {
    // 默认配置
    this.chunkSize = options.chunkSize || 512; // 每块最大字符数
    this.chunkOverlap = options.chunkOverlap || 50; // 块之间重叠字符数
    this.minChunkSize = options.minChunkSize || 50; // 最小块大小
  }

  /**
   * 将文本分块
   * @param {string} text - 输入文本
   * @returns {string[]} - 分块后的文本数组
   */
  chunk(text) {
    if (!text || text.trim().length === 0) {
      return [];
    }

    // 如果文本小于 chunkSize，直接返回
    if (text.length <= this.chunkSize) {
      return [text.trim()];
    }

    const chunks = [];
    let start = 0;

    while (start < text.length) {
      // 计算结束位置
      let end = start + this.chunkSize;

      // 如果已经到末尾，直接取剩余部分
      if (end >= text.length) {
        const chunk = text.slice(start).trim();
        if (chunk.length >= this.minChunkSize) {
          chunks.push(chunk);
        }
        break;
      }

      // 在句子边界处截断
      const chunkText = text.slice(start, end);
      const lastSentenceBreak = this.findSentenceBreak(chunkText);

      if (lastSentenceBreak > 0) {
        end = start + lastSentenceBreak;
      }

      const chunk = text.slice(start, end).trim();

      if (chunk.length >= this.minChunkSize) {
        chunks.push(chunk);
      }

      // 移动起始位置，考虑重叠
      start = end - this.chunkOverlap;
      if (start < 0) start = end;
    }

    return chunks;
  }

  /**
   * 查找句子边界
   * @param {string} text - 文本
   * @returns {number} - 最后一个句子边界的位置
   */
  findSentenceBreak(text) {
    // 中文句子边界：。！？.!?
    // 英文句子边界：.!?
    const separators = ['。', '！', '？', '.', '!', '?', '\n'];

    for (let i = text.length - 1; i >= 0; i--) {
      if (separators.includes(text[i])) {
        return i + 1;
      }
    }

    return 0;
  }

  /**
   * 按段落分块（适合结构化文档）
   */
  chunkByParagraph(text, options = {}) {
    const {
      paragraphSeparator = /\n\s*\n/,
      mergeSmall = true,
      minParagraphLength = 50,
    } = options;

    const paragraphs = text.split(paragraphSeparator);
    const chunks = [];
    let currentChunk = '';

    for (const para of paragraphs) {
      const trimmed = para.trim();
      if (!trimmed) continue;

      // 如果当前段落已经足够大，单独成块
      if (trimmed.length >= this.chunkSize) {
        if (currentChunk) {
          chunks.push(currentChunk.trim());
          currentChunk = '';
        }
        chunks.push(...this.chunk(trimmed));
        continue;
      }

      // 合并小段落
      if (mergeSmall) {
        if (currentChunk.length + trimmed.length > this.chunkSize) {
          chunks.push(currentChunk.trim());
          currentChunk = trimmed;
        } else {
          currentChunk += ' ' + trimmed;
        }
      } else {
        if (trimmed.length >= minParagraphLength) {
          chunks.push(trimmed);
        }
      }
    }

    // 处理剩余内容
    if (currentChunk) {
      chunks.push(currentChunk.trim());
    }

    return chunks;
  }

  /**
   * 配置更新
   */
  configure(options) {
    if (options.chunkSize) this.chunkSize = options.chunkSize;
    if (options.chunkOverlap) this.chunkOverlap = options.chunkOverlap;
    if (options.minChunkSize) this.minChunkSize = options.minChunkSize;
  }
}

// 单例模式
const chunkerService = new ChunkerService();
export default chunkerService;
