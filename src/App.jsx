import React, { useState, useEffect, useRef } from 'react';
import { Search, FileText, Command, RefreshCw, Sparkles, BrainCircuit, Pin, PinOff, FolderOpen, ChevronRight, ExternalLink, Clock, File, Plus, LayoutGrid } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

function App() {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState([]);
  const [indexedFiles, setIndexedFiles] = useState([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isSyncing, setIsSyncing] = useState(false);
  const [status, setStatus] = useState({ current: 0, total: 0, is_finished: false, watch_path: null });
  const [isSearching, setIsSearching] = useState(false);
  const [alwaysOnTop, setAlwaysOnTop] = useState(false);
  const [isPicking, setIsPicking] = useState(false);
  const inputRef = useRef(null);

  const activeResult = query.trim() ? results[selectedIndex] : null;
  const activeFile = !query.trim() ? indexedFiles[selectedIndex] : null;

  const handlePickFolder = async () => {
    setIsPicking(true);
    try { await invoke('pick_folder'); } catch (err) { alert(err); } finally { setIsPicking(false); }
  };

  const openInFinder = () => { if (status.watch_path) invoke('open_file', { path: status.watch_path }); };

  const highlightText = (text) => {
    if (!query.trim()) return text;
    const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`(${escapedQuery})`, 'gi');
    const parts = text.split(regex);
    return parts.map((part, i) => 
      part.toLowerCase() === query.toLowerCase() 
        ? <span key={i} className="highlight-text">{part}</span> 
        : part
    );
  };

  const fetchFiles = async () => {
    try {
      const files = await invoke('get_indexed_files');
      setIndexedFiles(files);
    } catch (err) { console.error(err); }
  };

  useEffect(() => {
    const updateStatus = async () => {
      try {
        const s = await invoke('get_indexing_status');
        setStatus(s);
        if (s.watch_path && !s.is_finished) setIsSyncing(true);
        if (s.is_finished) {
           setIsSyncing(false);
           if (!query.trim()) fetchFiles();
        }
      } catch (err) { console.error(err); }
    };
    updateStatus();
    const timer = setInterval(() => { if (!status.is_finished) updateStatus(); }, 1000);
    const unlisten = listen('indexing-finished', () => { updateStatus(); fetchFiles(); });
    return () => { clearInterval(timer); unlisten.then(f => f()); };
  }, [status.is_finished, query]);

  useEffect(() => {
    const doSearch = async () => {
      if (!query.trim()) { setResults([]); setSelectedIndex(0); fetchFiles(); return; }
      setIsSearching(true);
      try {
        const res = await invoke('search', { query });
        setResults(res);
        setSelectedIndex(0);
      } catch (err) { }
      finally { setIsSearching(false); }
    };
    const t = setTimeout(doSearch, 200);
    return () => clearTimeout(t);
  }, [query]);

  useEffect(() => {
    const handleKeyDown = (e) => {
      const max = query.trim() ? results.length : indexedFiles.length;
      if (e.key === 'ArrowDown') { setSelectedIndex((p) => (p + 1) % (max || 1)); e.preventDefault(); }
      else if (e.key === 'ArrowUp') { setSelectedIndex((p) => (p - 1 + max) % (max || 1)); e.preventDefault(); }
      else if (e.key === 'Enter') {
        const p = query.trim() ? results[selectedIndex]?.file_path : indexedFiles[selectedIndex]?.path;
        if (p) invoke('open_file', { path: p });
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [results, indexedFiles, selectedIndex, query]);

  if (!status.watch_path && !isSyncing) {
    return (
      <div className="flex flex-col h-screen bg-white items-center justify-center p-12 text-center rounded-[2.5rem] border border-gray-100 shadow-2xl overflow-hidden">
        <div onMouseDown={() => invoke('start_dragging')} className="absolute top-0 left-0 right-0 h-16 cursor-grab" />
        <div className="w-24 h-24 bg-blue-50 rounded-[2.5rem] flex items-center justify-center mb-8 shadow-inner shadow-blue-100/50">
           <FolderOpen size={48} className={isPicking ? "text-blue-300 animate-pulse" : "text-blue-500"} />
        </div>
        <h1 className="text-3xl font-bold text-gray-800 mb-4 tracking-tight">VaultSeek 语义索引</h1>
        <p className="text-gray-400 mb-10 max-w-sm leading-relaxed">选择您的知识库文件夹，VaultSeek 将在本地为您开启 AI 检索新体验。</p>
        <button onClick={handlePickFolder} disabled={isPicking} className="bg-blue-600 text-white px-10 py-5 rounded-2xl font-bold shadow-2xl shadow-blue-200 transition-all active:scale-95 flex items-center gap-3 text-lg">
          {isPicking ? <RefreshCw className="animate-spin" size={24} /> : <Plus size={24} />}
          立即关联文件夹
        </button>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-white/95 backdrop-blur-3xl rounded-[2.5rem] border border-white/60 shadow-[0_50px_100px_rgba(0,0,0,0.1)] overflow-hidden">
      <div onMouseDown={() => invoke('start_dragging')} className="absolute top-0 left-0 right-0 h-16 z-[100] cursor-grab active:cursor-grabbing" />
      
      {/* 左侧侧边栏 */}
      <div className="w-[38%] flex flex-col border-r border-gray-100 bg-gray-50/30">
        <div className="px-6 pt-12 pb-4 space-y-5">
          {/* 库管理头部 */}
          <div className="flex items-center justify-between">
             <div className="flex items-center gap-2 text-gray-800">
               <div className="p-2 bg-white rounded-xl shadow-sm border border-gray-100"><LayoutGrid size={16} className="text-blue-500" /></div>
               <span className="text-[14px] font-black tracking-tight truncate max-w-[140px]">
                 {status.watch_path?.split('/').pop() || '我的知识库'}
               </span>
             </div>
             <div className="flex items-center gap-1">
                <button onClick={openInFinder} title="在访达中打开" className="p-2 text-gray-400 hover:text-blue-500 hover:bg-white rounded-lg transition-all"><FolderOpen size={16} /></button>
                <button onClick={handlePickFolder} title="添加/更换文件夹" className="p-2 bg-blue-600 text-white hover:bg-blue-700 rounded-lg shadow-lg shadow-blue-100 transition-all"><Plus size={16} /></button>
             </div>
          </div>

          {/* 搜索框 */}
          <div className="flex items-center gap-3 bg-white border border-gray-200/60 rounded-2xl px-4 py-3.5 shadow-sm focus-within:ring-2 focus-within:ring-blue-500/20 transition-all">
            {isSyncing ? <BrainCircuit className="text-blue-500 animate-pulse" size={20} /> : <Search className="text-gray-400" size={20} />}
            <input ref={inputRef} autoFocus className="flex-1 bg-transparent outline-none text-[15px] text-gray-800 placeholder-gray-300 font-medium" placeholder={isSyncing ? `解析中... ${status.current}/${status.total}` : "搜索或浏览文件..."} value={query} onChange={(e) => setQuery(e.target.value)} />
          </div>
        </div>

        {/* 列表区 */}
        <div className="flex-1 overflow-y-auto px-3 pb-4 space-y-1">
          {query.trim() ? (
            results.map((res, i) => (
              <div key={res.id} onMouseEnter={() => setSelectedIndex(i)} onClick={() => invoke('open_file', { path: res.file_path })} className={`group flex items-start gap-3 p-3.5 rounded-2xl cursor-pointer transition-all ${i === selectedIndex ? 'bg-blue-600 shadow-lg shadow-blue-200 scale-[1.02]' : 'hover:bg-white'}`}>
                <div className={`shrink-0 p-2.5 rounded-xl ${i === selectedIndex ? 'bg-white/20' : 'bg-blue-50'}`}><FileText size={18} className={i === selectedIndex ? 'text-white' : 'text-blue-500'} /></div>
                <div className="flex-1 min-w-0">
                  <div className={`text-[14px] font-bold truncate ${i === selectedIndex ? 'text-white' : 'text-gray-800'}`}>{res.file_name}</div>
                  <div className={`text-[11px] truncate mt-0.5 opacity-60 ${i === selectedIndex ? 'text-blue-100' : 'text-gray-400'}`}>{res.file_path.split('/').slice(-2).join('/')}</div>
                  <div className={`text-[12px] line-clamp-1 mt-1 ${i === selectedIndex ? 'text-blue-50/80' : 'text-gray-400'}`}>{highlightText(res.content)}</div>
                </div>
              </div>
            ))
          ) : (
            indexedFiles.map((file, i) => (
              <div key={file.path} onMouseEnter={() => setSelectedIndex(i)} onClick={() => invoke('open_file', { path: file.path })} className={`group flex items-center gap-3 p-3.5 rounded-2xl cursor-pointer transition-all ${i === selectedIndex ? 'bg-gray-200/60 shadow-sm' : 'hover:bg-white/60'}`}>
                <div className={`p-2 rounded-lg ${i === selectedIndex ? 'bg-blue-500 text-white' : 'bg-white text-gray-400 shadow-sm border border-gray-100'}`}><File size={14} /></div>
                <div className="flex-1 min-w-0">
                  <div className={`text-[13px] font-bold truncate ${i === selectedIndex ? 'text-gray-900' : 'text-gray-600'}`}>{file.name}</div>
                  <div className="text-[10px] text-gray-400 truncate opacity-70">{file.path.split('/').slice(-2, -1)}</div>
                </div>
                <ChevronRight size={14} className={`opacity-0 group-hover:opacity-100 transition-opacity ${i === selectedIndex ? 'text-blue-500' : 'text-gray-300'}`} />
              </div>
            ))
          )}
        </div>

        {/* 底部状态 */}
        <div className="px-6 py-4 border-t border-gray-100 flex items-center justify-between text-[10px] font-bold text-gray-400 uppercase tracking-tighter bg-white/50">
          <div className="flex items-center gap-1.5">
            <div className={`w-1.5 h-1.5 rounded-full ${isSyncing ? 'bg-blue-500 animate-ping' : 'bg-green-500'}`} />
            {isSyncing ? 'Indexing' : 'Local Ready'}
          </div>
          <span className="bg-gray-100 px-2 py-0.5 rounded-md">{indexedFiles.length} FILES</span>
        </div>
      </div>

      {/* 右侧：预览区 */}
      <div className="flex-1 flex flex-col bg-white">
        {activeResult || activeFile ? (
          <div className="flex flex-col h-full">
            <div className="px-10 pt-12 pb-6 border-b border-gray-50">
              <div className="flex items-center gap-2 text-blue-500 font-bold text-[11px] uppercase tracking-widest mb-3"><Clock size={12} /> <span>{query.trim() ? 'Match Fragment' : 'File Preview'}</span></div>
              <h2 className="text-3xl font-black text-gray-800 leading-tight mb-5">{activeResult?.file_name || activeFile?.name}</h2>
              <div className="flex gap-3">
                <button onClick={() => invoke('open_file', { path: activeResult?.file_path || activeFile?.path })} className="flex items-center gap-2 px-5 py-2.5 bg-gray-900 text-white rounded-xl text-xs font-bold hover:bg-blue-600 transition-all shadow-xl shadow-gray-200 active:scale-95">
                  <ExternalLink size={14} /> 打开原文
                </button>
                <div className="flex items-center gap-2 px-4 py-2 bg-gray-50 text-gray-400 rounded-xl text-[11px] border border-gray-100 truncate max-w-[300px]">
                  <File size={12} /> {activeResult?.file_path || activeFile?.path}
                </div>
              </div>
            </div>
            <div className="flex-1 overflow-y-auto px-10 py-10 bg-white">
              <div className="max-w-2xl mx-auto">
                <div className="text-[18px] leading-[1.85] text-gray-700 space-y-8 font-normal">
                  {(activeResult?.content || "正在加载内容预览，点击上方按钮可直接编辑源文件...").split('\n').map((para, idx) => <p key={idx} className="mb-6">{highlightText(para)}</p>)}
                </div>
                <div className="mt-20 pt-10 border-t border-gray-50 text-gray-300 text-center">
                  <p className="text-[11px] uppercase tracking-[0.2em] font-bold opacity-50 italic">Private Knowledge Fragment · Encrypted Local Storage</p>
                </div>
              </div>
            </div>
          </div>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center p-20 text-center bg-gray-50/20">
            <div className="w-40 h-40 bg-white rounded-[3.5rem] flex items-center justify-center mb-8 shadow-xl shadow-gray-100 border border-gray-100/50">
              <Command size={56} className="text-blue-500/20" />
            </div>
            <h3 className="text-2xl font-bold text-gray-800 mb-3">知识唤醒中</h3>
            <p className="text-gray-400 text-sm max-w-xs leading-relaxed font-medium">请从左侧列表选择文件，或在搜索框输入您想找的内容。VaultSeek 会实时在本地为您匹配最精准的语义片段。</p>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;
