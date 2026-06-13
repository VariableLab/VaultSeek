import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function BatchExportModal({ isOpen, onClose, onResult }) {
  const [libraries, setLibraries] = useState([]);
  const [selectedPaths, setSelectedPaths] = useState([]);
  const [query, setQuery] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);

  useEffect(() => {
    if (isOpen) {
      invoke('get_all_libraries').then(setLibraries);
    }
  }, [isOpen]);

  const togglePath = (path) => {
    setSelectedPaths(prev => 
      prev.includes(path) ? prev.filter(p => p !== path) : [...prev, path]
    );
  };

  const handleExport = async () => {
    if (selectedPaths.length === 0 || !query) return;
    setIsProcessing(true);
    try {
      const report = await invoke('batch_export_reports', { paths: selectedPaths, query });
      onResult(report);
      onClose();
    } catch (err) {
      alert("导出失败: " + err);
    } finally {
      setIsProcessing(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-[200] flex items-center justify-center">
      <div className="bg-[#18181b] p-6 rounded-xl border border-neutral-700 w-[500px]">
        <h2 className="text-lg font-bold mb-4">跨库批量检索</h2>
        <div className="mb-4">
          <input 
            value={query} onChange={(e) => setQuery(e.target.value)}
            className="w-full bg-[#2d2d2d] border border-neutral-700 rounded-lg p-2 text-sm text-white"
            placeholder="输入分析主题..."
          />
        </div>
        <div className="space-y-2 mb-4 max-h-60 overflow-y-auto">
          {libraries.map(path => (
            <label key={path} className="flex items-center gap-2 text-sm">
              <input type="checkbox" checked={selectedPaths.includes(path)} onChange={() => togglePath(path)} />
              {path}
            </label>
          ))}
        </div>
        <div className="flex justify-end gap-2">
          <button onClick={onClose} className="px-4 py-2 bg-neutral-700 rounded-lg text-sm">取消</button>
          <button onClick={handleExport} disabled={isProcessing} className="px-4 py-2 bg-blue-600 rounded-lg text-sm">
            {isProcessing ? '处理中...' : '开始导出'}
          </button>
        </div>
      </div>
    </div>
  );
}
