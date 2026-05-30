import React, { useState, useEffect, useRef } from 'react';
import { Search, FileText, Command, RefreshCw, Sparkles, BrainCircuit, Pin, PinOff, FolderOpen, AlertCircle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

function App() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isSyncing, setIsSyncing] = useState(false);
  const [status, setStatus] = useState({ current: 0, total: 0, is_finished: false, watch_path: null });
  const [isSearching, setIsSearching] = useState(false);
  const [totalFiles, setTotalFiles] = useState(0);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const inputRef = useRef(null);

  // 1. 核心状态轮询：获取当前配置和索引状态
  useEffect(() => {
    const updateStatus = async () => {
      try {
        const s = await invoke('get_indexing_status');
        setStatus(s);
        
        // 如果有路径且正在同步
        if (s.watch_path && !s.is_finished) {
          setIsSyncing(true);
        }

        // 如果索引完成，拉取文件列表
        if (s.is_finished && s.watch_path) {
          const files = await invoke('get_files');
          setTotalFiles(files.length);
          setIsSyncing(false);
        }
      } catch (err) { console.error(err); }
    };

    updateStatus();
    const timer = setInterval(() => { if (!status.is_finished) updateStatus(); }, 1000);
    const unlisten = listen('indexing-finished', () => { 
      console.log('索引大功告成');
      updateStatus(); 
    });

    return () => { clearInterval(timer); unlisten.then(f => f()); };
  }, [status.is_finished]);

  // 2. 语义搜索
  useEffect(() => {
    const doSearch = async () => {
      if (query.trim() === '' || isSyncing || !status.watch_path) {
        setResults([]);
        setIsSearching(false);
        return;
      }
      setIsSearching(true);
      try {
        const res = await invoke('search', { query });
        setResults(res);
        setSelectedIndex(0);
      } catch (err) { }
      finally { setIsSearching(false); }
    };
    const t = setTimeout(doSearch, 250);
    return () => clearTimeout(t);
  }, [query, isSyncing, status.watch_path]);

  // 3. 处理键盘导航
  useEffect(() => {
    const handleKeyDown = (e) => {
      if (e.key === 'ArrowDown') {
        setSelectedIndex((prev) => (prev + 1) % (results.length || 1));
        e.preventDefault();
      } else if (e.key === 'ArrowUp') {
        setSelectedIndex((prev) => (prev - 1 + results.length) % (results.length || 1));
        e.preventDefault();
      } else if (e.key === 'Enter' && results[selectedIndex]) {
        invoke('open_file', { path: results[selectedIndex].path });
      } else if (e.key === 'Escape') {
        invoke('hide_window');
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, selectedIndex]);

  const handlePickFolder = async () => {
    try {
      const path = await invoke('pick_folder');
      if (path) {
        setIsSyncing(true);
        // 重新触发状态更新
        const s = await invoke('get_indexing_status');
        setStatus(s);
      }
    } catch (err) { console.error(err); }
  };

  const togglePin = async () => {
    const newVal = !alwaysOnTop;
    setAlwaysOnTop(newVal);
    await invoke('set_always_on_top', { onTop: newVal });
  };

  // ========== UI 渲染逻辑 ==========

  // 第一步：如果还没选过文件夹
  if (!status.watch_path && !isSyncing) {
    return (
      <div className="flex flex-col h-screen bg-white/95 backdrop-blur-3xl rounded-[2.5rem] border border-white/60 shadow-[0_50px_150px_rgba(0,0,0,0.6)] overflow-hidden items-center justify-center p-12 text-center">
        <div onMouseDown={() => invoke('start_dragging')} className="absolute top-0 left-0 right-0 h-14 cursor-grab" />
        <div className="w-24 h-24 bg-blue-50 rounded-full flex items-center justify-center mb-8 animate-bounce">
           <FolderOpen size={48} className="text-blue-500" />
        </div>
        <h1 className="text-3xl font-black text-gray-800 mb-4 tracking-tight">欢迎使用 VaultSeek RAG</h1>
        <p className="text-gray-400 text-lg font-light mb-10 leading-relaxed max-w-md">
          要开启本地 AI 语义搜索，请先选择一个包含 Markdown 或 PDF 的文件夹。
        </p>
        <button 
          onClick={handlePickFolder}
          className="flex items-center gap-3 bg-blue-600 hover:bg-blue-700 text-white px-10 py-5 rounded-2xl text-xl font-bold transition-all shadow-xl shadow-blue-200 active:scale-95"
        >
          <FolderOpen size={24} /> 选择监控文件夹
        </button>
      </div>
    );
  }

  // 第二步：正常搜索界面
  return (
    <div className="relative flex flex-col h-screen bg-white/95 backdrop-blur-3xl rounded-[2.5rem] border border-white/60 shadow-[0_50px_150px_rgba(0,0,0,0.6)] overflow-hidden">
      
      {/* 强化版拖拽手柄：绝对定位 z-index 最高 */}
      <div 
        onMouseDown={() => invoke('start_dragging')}
        className="absolute top-0 left-0 right-0 h-16 z-[100] cursor-grab active:cursor-grabbing flex items-start justify-center pt-4"
      >
        <div className="w-16 h-1.5 bg-gray-200/50 rounded-full hover:bg-blue-200 transition-colors" />
      </div>

      <div className="flex items-center px-10 pt-12 pb-4 relative z-40">
        <div className="relative flex items-center justify-center w-10 h-10">
          {isSyncing ? (
            <BrainCircuit className="text-blue-500 animate-pulse" size={32} />
          ) : (
            <>
              <Search className={`absolute transition-all duration-500 ${isSearching ? 'opacity-0 scale-50' : 'text-gray-400'}`} size={32} />
              {isSearching && <Sparkles className="absolute text-blue-500 animate-spin-slow" size={32} />}
            </>
          )}
        </div>
        <input
          ref={inputRef}
          autoFocus
          className="flex-1 bg-transparent outline-none text-2xl text-gray-800 placeholder-gray-300 font-light ml-6"
          placeholder={isSyncing ? `RAG 正在理解知识 (${status.current}/${status.total})` : "快速定位笔记或 PDF..."}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="flex gap-2">
          <button onClick={handlePickFolder} className="p-2 text-gray-300 hover:text-blue-500 transition-colors" title="更改文件夹"><FolderOpen size={18} /></button>
          <button 
            onClick={togglePin}
            className={`p-2 rounded-xl transition-all duration-300 ${alwaysOnTop ? 'bg-blue-500 text-white' : 'bg-gray-50 text-gray-400 hover:bg-gray-100'}`}
          >
            {alwaysOnTop ? <Pin size={18} /> : <PinOff size={18} />}
          </button>
        </div>
      </div>

      {isSyncing && (
        <div className="px-10 mt-1 mb-2">
          <div className="h-1 w-full bg-gray-100 rounded-full overflow-hidden">
            <div 
              className="h-full bg-blue-500 transition-all duration-500"
              style={{ width: `${(status.current / status.total) * 100 || 0}%` }}
            />
          </div>
        </div>
      )}

      <div className="flex-1 overflow-y-auto px-6 py-4 relative z-40">
        {results.length > 0 ? (
          <div className="space-y-3">
            {results.map((res, index) => (
              <div
                key={res.id}
                onClick={() => invoke('open_file', { path: res.path })}
                className={`flex items-center px-6 py-5 rounded-[1.8rem] cursor-pointer transition-all duration-300 ${
                  index === selectedIndex ? 'bg-blue-600 shadow-2xl scale-[1.02] -translate-y-1' : 'hover:bg-gray-50'
                }`}
              >
                <div className={`p-3 rounded-2xl mr-6 ${index === selectedIndex ? 'bg-white/20' : 'bg-blue-50'}`}>
                  <FileText className={index === selectedIndex ? 'text-white' : 'text-blue-500'} size={28} />
                </div>
                <div className="flex-1 min-w-0">
                  <div className={`text-xl font-bold truncate mb-1 ${index === selectedIndex ? 'text-white' : 'text-gray-800'}`}>
                    {res.name}
                  </div>
                  <div className={`text-sm truncate leading-relaxed ${index === selectedIndex ? 'text-blue-50 opacity-90' : 'text-gray-400 font-light'}`}>
                    {res.content_preview}...
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center h-full text-gray-300">
            <Sparkles size={64} className={isSearching || isSyncing ? 'text-blue-100 animate-pulse' : 'opacity-10'} />
            <p className="mt-4 text-xl font-light text-center px-10">
              {isSyncing ? 'AI 正在阅读本地硬盘内容...' : query ? '未找到相关匹配' : 'VaultSeek RAG 随时待命'}
            </p>
          </div>
        )}
      </div>

      <div className="px-10 py-6 bg-gray-50/50 flex justify-between items-center relative z-40 border-t border-gray-100/50">
        <div className="flex gap-8 text-[12px] font-bold tracking-widest text-gray-400 uppercase">
          <span>{totalFiles} DOCUMENTS</span>
          <span className="flex items-center gap-2">
            <div className={`w-2.5 h-2.5 rounded-full ${isSyncing ? 'bg-blue-500 animate-ping' : 'bg-green-500'}`} />
            {isSyncing ? 'INDEXING' : 'AI READY'}
          </span>
        </div>
        <div className="text-[10px] text-gray-400 truncate max-w-[200px]" title={status.watch_path}>
          PATH: {status.watch_path}
        </div>
      </div>
    </div>
  );
}

export default App;
