/**
 * Storage Service
 * 在 Electron 环境使用 IPC 调用主进程 SQLite
 * 在浏览器环境使用 localStorage 作为降级方案
 */

const STORAGE_KEY = 'vaultseek-documents';
const STORAGE_KEY_VECTORS = 'vaultseek-vectors-compressed';
const STORAGE_KEY_SETTINGS = 'vaultseek-settings';

const isElectron = () => {
  return typeof window !== 'undefined' && window.electronAPI;
};

class StorageService {
  constructor() {
    this.initialized = false;
    this.useElectron = false;
  }

  async init() {
    if (this.initialized) {
      return;
    }

    this.useElectron = isElectron();
    console.log('[Storage] 初始化完成，环境:', this.useElectron ? 'Electron' : 'Browser');
    this.initialized = true;
  }

  // ============ 文档管理 ============

  async getDocuments() {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      const result = await window.electronAPI.getDocuments();
      return result.success ? result.documents : [];
    }

    // 浏览器降级方案
    const stored = localStorage.getItem(STORAGE_KEY);
    return stored ? JSON.parse(stored) : [];
  }

  async addDocument(doc) {
    if (!this.initialized) await this.init();

    const document = {
      id: doc.id || `doc-${Date.now()}`,
      filename: doc.filename || doc.name,
      filetype: doc.filetype || doc.type || 'unknown',
      filesize: doc.filesize || doc.size || 0,
      filepath: doc.path || null,
      content: doc.content || null,
      chunk_count: 0,
      status: 'pending',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    if (this.useElectron) {
      await window.electronAPI.importDocument(document);
    } else {
      // 浏览器降级方案
      const documents = await this.getDocuments();
      documents.push(document);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(documents));
    }

    return document;
  }

  async addDocuments(docs) {
    const results = [];
    for (const doc of docs) {
      const result = await this.addDocument(doc);
      results.push(result);
    }
    return results;
  }

  async updateDocumentStatus(id, status, errorMessage = null) {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      await window.electronAPI.updateDocumentStatus(id, status, errorMessage);
    } else {
      const documents = await this.getDocuments();
      const index = documents.findIndex((d) => d.id === id);
      if (index !== -1) {
        documents[index].status = status;
        documents[index].error = errorMessage;
        localStorage.setItem(STORAGE_KEY, JSON.stringify(documents));
      }
    }
  }

  async deleteDocument(id) {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      await window.electronAPI.deleteDocument(id);
    } else {
      const documents = await this.getDocuments();
      const filtered = documents.filter((d) => d.id !== id);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(filtered));
    }
  }

  async clearDocuments() {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      await window.electronAPI.clearAllData();
    } else {
      localStorage.setItem(STORAGE_KEY, '[]');
    }
  }

  // ============ 向量管理 ============

  async getVectors() {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      const result = await window.electronAPI.getVectors();
      return result.success ? result.vectors : [];
    }

    // 浏览器降级方案
    const stored = localStorage.getItem(STORAGE_KEY_VECTORS);
    return stored ? JSON.parse(stored) : [];
  }

  async saveVectors(vectors) {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      const result = await window.electronAPI.saveVectors(vectors);
      return result;
    } else {
      // 浏览器降级方案
      localStorage.setItem(STORAGE_KEY_VECTORS, JSON.stringify(vectors));
      return { success: true };
    }
  }

  async clearVectors() {
    if (!this.initialized) await this.init();
    localStorage.setItem(STORAGE_KEY_VECTORS, '[]');
  }

  // ============ 配置管理 ============

  async getConfig(key) {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      const result = await window.electronAPI.getConfig(key);
      return result.success ? result.value : null;
    }

    // 浏览器降级方案
    const stored = localStorage.getItem(STORAGE_KEY_SETTINGS);
    const settings = stored ? JSON.parse(stored) : {};
    return settings[key] || null;
  }

  async setConfig(key, value) {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      await window.electronAPI.setConfig(key, value);
    } else {
      const stored = localStorage.getItem(STORAGE_KEY_SETTINGS);
      const settings = stored ? JSON.parse(stored) : {};
      settings[key] = value;
      localStorage.setItem(STORAGE_KEY_SETTINGS, JSON.stringify(settings));
    }
  }

  async getAllConfigs() {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      const result = await window.electronAPI.getAllConfigs();
      return result.success ? result.configs : {};
    }

    const stored = localStorage.getItem(STORAGE_KEY_SETTINGS);
    return stored ? JSON.parse(stored) : {};
  }

  // ============ 使用统计 ============

  async getUsageStats() {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      const result = await window.electronAPI.getUsageStats();
      return result;
    }

    // 浏览器降级方案
    const today = new Date().toISOString().split('T')[0];
    const stored = localStorage.getItem(`vaultseek-usage-${today}`);
    const data = stored ? JSON.parse(stored) : { llm_count: 0, search_count: 0 };

    return {
      success: true,
      today: data.llm_count || 0,
      searchCount: data.search_count || 0,
      limit: 10,
    };
  }

  async checkUsage() {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      return await window.electronAPI.checkUsage();
    }

    // 浏览器降级方案
    const stats = await this.getUsageStats();
    return {
      hasRemaining: stats.today < stats.limit,
      used: stats.today,
      limit: stats.limit,
      remaining: Math.max(0, stats.limit - stats.today),
    };
  }

  async incrementUsage(type = 'llm') {
    if (!this.initialized) await this.init();

    if (this.useElectron) {
      return await window.electronAPI.incrementUsage(type);
    }

    // 浏览器降级方案
    const today = new Date().toISOString().split('T')[0];
    const stored = localStorage.getItem(`vaultseek-usage-${today}`);
    const data = stored ? JSON.parse(stored) : { llm_count: 0, search_count: 0 };

    if (type === 'llm') {
      data.llm_count += 1;
    } else if (type === 'search') {
      data.search_count += 1;
    }

    localStorage.setItem(`vaultseek-usage-${today}`, JSON.stringify(data));
    return { success: true };
  }

  // ============ 统计信息 ============

  async getStats() {
    const docs = await this.getDocuments();
    const completedDocs = docs.filter((d) => d.status === 'completed' || d.status === '已完成');

    return {
      docCount: docs.length,
      completedCount: completedDocs.length,
      vectorCount: 0, // 由 vectorIndex 服务管理
      storageUsed: this.calculateStorage(docs),
    };
  }

  calculateStorage(docs) {
    const total = docs.reduce((sum, d) => {
      const size = d.filesize || d.size || 0;
      if (typeof size === 'number') {
        return sum + size;
      }
      const match = (d.size || '0 B').match(/([\d.]+)/);
      const num = match ? parseFloat(match[1]) : 0;
      if ((d.size || '').includes('GB')) return sum + num * 1024;
      if ((d.size || '').includes('MB')) return sum + num;
      if ((d.size || '').includes('KB')) return sum + num / 1024;
      return sum;
    }, 0);
    return total.toFixed(2) + ' MB';
  }

  async getRecentDocs(limit = 10) {
    const docs = await this.getDocuments();
    return docs.slice(-limit).reverse();
  }

  isReady() {
    return this.initialized;
  }
}

// 单例模式
const storageService = new StorageService();
export default storageService;
