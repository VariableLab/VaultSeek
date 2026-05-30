/**
 * Document Parser Service
 * 支持 PDF, Word, TXT 格式解析
 */

class ParserService {
  constructor() {
    this.supportedFormats = ['pdf', 'doc', 'docx', 'txt', 'md'];
  }

  /**
   * 解析文件
   * @param {File} file - File 对象
   * @returns {Promise<{text: string, metadata: object}>}
   */
  async parse(file) {
    const extension = file.name.split('.').pop().toLowerCase();

    if (!this.supportedFormats.includes(extension)) {
      throw new Error(`不支持的文件格式：${extension}`);
    }

    try {
      let text = '';

      if (extension === 'pdf') {
        text = await this.parsePDF(file);
      } else if (extension === 'docx' || extension === 'doc') {
        text = await this.parseWord(file);
      } else if (extension === 'txt' || extension === 'md') {
        text = await this.parseText(file);
      }

      return {
        text,
        metadata: {
          filename: file.name,
          extension,
          size: file.size,
          lastModified: file.lastModified,
        },
      };
    } catch (error) {
      console.error('[Parser] 解析失败:', error);
      throw new Error(`文件解析失败：${error.message}`);
    }
  }

  /**
   * 解析 PDF 文件
   */
  async parsePDF(file) {
    const { pdf } = await import('pdf-parse/lib/pdf-parse.js');
    const arrayBuffer = await file.arrayBuffer();
    const data = await pdf(new Uint8Array(arrayBuffer));
    return data.text;
  }

  /**
   * 解析 Word 文件
   */
  async parseWord(file) {
    const mammoth = await import('mammoth');
    const arrayBuffer = await file.arrayBuffer();

    if (file.name.endsWith('.doc')) {
      throw new Error('仅支持 .docx 格式，请将 .doc 转换为 .docx');
    }

    const result = await mammoth.extractRawText({ arrayBuffer });
    return result.value;
  }

  /**
   * 解析文本文件
   */
  async parseText(file) {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = (e) => resolve(e.target.result);
      reader.onerror = (e) => reject(e);
      reader.readAsText(file, 'utf-8');
    });
  }

  /**
   * 检查文件格式是否支持
   */
  isSupported(filename) {
    const extension = filename.split('.').pop().toLowerCase();
    return this.supportedFormats.includes(extension);
  }

  /**
   * 获取支持的文件格式列表
   */
  getSupportedFormats() {
    return [...this.supportedFormats];
  }
}

// 单例模式
const parserService = new ParserService();
export default parserService;
