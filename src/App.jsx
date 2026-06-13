import React, { useReducer, useEffect, useRef, useState, memo } from 'react';
import { Search, FolderOpen, Settings, Send, Bot, Loader2, Sparkles, LayoutGrid, Zap, Database, Server, MessageSquare, ExternalLink } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import SettingsModal from './components/SettingsModal';

// --- 子组件提取与性能优化 ---

// --- 辅助函数：正则表达式转义 ---
const escapeRegExp = (string) => {
  return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
};

// --- 辅助函数：关键词高亮 ---
const HighlightText = ({ text, keywords }) => {
  if (!text) return null;
  if (!keywords || (Array.isArray(keywords) && keywords.length === 0)) return <span>{text}</span>;
  
  try {
    const rawWords = Array.isArray(keywords) ? keywords : [keywords];
    const words = rawWords
      .flatMap(w => typeof w === 'string' ? w.split(/[,\s，、]+/) : [])
      .map(w => w.trim())
      .filter(w => w.length >= 2);

    if (words.length === 0) return <span>{text}</span>;

    const uniqueWords = [...new Set(words)];
    const pattern = uniqueWords.map(w => escapeRegExp(w)).join('|');
    const regex = new RegExp(`(${pattern})`, 'gi');
    
    const parts = text.split(regex);
    return (
      <span>
        {parts.map((part, i) => 
          regex.test(part) ? (
            <mark key={i} className="bg-yellow-500/30 text-yellow-200 rounded px-0.5 border-b border-yellow-500/50">
              {part}
            </mark>
          ) : (
            part
          )
        )}
      </span>
    );
  } catch (err) {
    return <span>{text}</span>;
  }
};

// --- 辅助函数：结构化文本渲染 ---
const StructuredContent = ({ text, keywords }) => {
  if (!text) return null;
  const lines = text.split('\n');
  return (
    <div className="space-y-4">
      {lines.map((line, idx) => {
        const trimmed = line.trim();
        if (!trimmed) return <div key={idx} className="h-2" />;
        if (trimmed.startsWith('## ')) return <h2 key={idx} className="text-lg font-bold text-white mt-6 mb-2 border-b border-neutral-800 pb-1"><HighlightText text={trimmed.replace('## ', '')} keywords={keywords} /></h2>;
        if (trimmed.startsWith('### ')) return <h3 key={idx} className="text-md font-bold text-blue-400 mt-4 mb-2"><HighlightText text={trimmed.replace('### ', '')} keywords={keywords} /></h3>;
        if (trimmed.startsWith('- ') || trimmed.startsWith('* ') || /^\d+\./.test(trimmed)) {
          const content = trimmed.replace(/^[-*] |\d+\. /, '');
          return <div key={idx} className="flex gap-2 ml-2 mb-1"><span className="text-blue-500 mt-1.5">•</span><span className="flex-1"><HighlightText text={content} keywords={keywords} /></span></div>;
        }
        return <p key={idx} className="leading-relaxed"><HighlightText text={line} keywords={keywords} /></p>;
      })}
    </div>
  );
};

const ChatMessage = memo(({ msg, isLast, keywords }) => {
  return (
    <div className={`flex gap-6 ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
      {msg.role === 'assistant' && (
        <div className="w-9 h-9 rounded-xl bg-gradient-to-br from-indigo-600 to-blue-700 flex items-center justify-center shrink-0 shadow-lg shadow-blue-900/20">
          <Bot size={20} className="text-white" />
        </div>
      )}
      <div className={`max-w-[88%] ${
        msg.role === 'user' 
          ? 'bg-blue-600 text-white px-5 py-3 rounded-2xl rounded-tr-none shadow-lg shadow-blue-900/20 text-sm' 
          : 'prose-ai border-l-2 border-blue-500/30 pl-6 py-1'
      }`}>
        {msg.role === 'user' ? <HighlightText text={msg.content} keywords={keywords} /> : <StructuredContent text={msg.content} keywords={keywords} />}
      </div>
      {msg.role === 'user' && <div className="w-9 h-9 rounded-xl bg-neutral-800 border border-neutral-700 flex items-center justify-center shrink-0"><span className="text-[10px] font-bold text-neutral-400">YOU</span></div>}
    </div>
  );
});

const ReferenceList = memo(({ references, query, selectedIds, onToggle }) => {
  if (!references || references.length === 0) return null;
  return (
    <div className="mt-12 ml-14 space-y-6">
      <div className="text-[10px] font-black text-neutral-600 uppercase tracking-[0.3em] flex items-center gap-4">
         <div className="h-[1px] w-8 bg-neutral-800" />
         Evidence Vault
         <div className="h-[1px] flex-1 bg-neutral-800" />
      </div>
      <div className="grid grid-cols-1 gap-4">
        {references.map((ref, idx) => {
          const ext = ref.file_name.split('.').pop().toUpperCase();
          const isSelected = selectedIds?.has(ref.id);
          return (
            <div key={idx} className={`archive-card group p-5 border rounded-xl transition-all duration-300 ${isSelected ? 'border-blue-500/50 bg-blue-900/10' : 'border-neutral-800/30'}`}>
              <div className="flex items-start justify-between mb-4">
                <div className="flex items-start gap-4 cursor-pointer" onClick={() => onToggle?.(ref.id)}>
                   <div className={`mt-1 w-4 h-4 rounded border flex items-center justify-center transition-colors ${isSelected ? 'bg-blue-600 border-blue-600' : 'border-neutral-700 group-hover:border-neutral-500'}`}>
                      {isSelected && <Zap size={10} className="text-white fill-current" />}
                   </div>
                   <div className="flex flex-col gap-1">
                    <div className="flex items-center gap-2">
                      <span className="px-1.5 py-0.5 rounded bg-neutral-800 text-[9px] font-bold text-neutral-400 border border-neutral-700">{ext}</span>
                      <span className="text-[11px] font-bold text-neutral-300 group-hover:text-blue-400 transition-colors truncate max-w-[300px]">{ref.file_name}</span>
                    </div>
                    <div className="text-[9px] text-neutral-600 font-mono tracking-tighter">REF-ID: {ref.file_path.slice(-8)}</div>
                  </div>
                </div>
                <div onClick={(e) => { e.stopPropagation(); invoke('open_file', { path: ref.file_path }); }} className="p-2 hover:bg-neutral-800 rounded-lg cursor-pointer text-neutral-500 hover:text-white transition-all"><ExternalLink size={14} /></div>
              </div>
              <div className="prose-source line-clamp-5 relative">
                <HighlightText text={ref.content} keywords={query} />
                <div className="absolute bottom-0 left-0 w-full h-8 bg-gradient-to-t from-[#121214] to-transparent opacity-20" />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
});

const ThinkingChain = () => {
  const [step, setStep] = useState(0);
  const steps = ["分析语义意图...", "检索本地知识档案...", "提炼核心资产...", "构建关联报告..."];
  useEffect(() => {
    const timer = setInterval(() => setStep(s => (s + 1) % steps.length), 2000);
    return () => clearInterval(timer);
  }, []);
  return (
    <div className="flex items-center gap-4 text-neutral-500 py-2">
       <Loader2 size={16} className="animate-spin text-blue-500" />
       <div className="text-xs font-medium tracking-wide animate-pulse">{steps[step]}</div>
    </div>
  );
};

// --- 状态管理 ---

const initialState = {
  status: 'IDLE',
  query: '',
  chatHistory: [],
  currentAssistantMessage: '',
  references: [],
  indexingStatus: { current: 0, total: 0, is_finished: true, watch_path: null },
  selectedSourceIds: new Set(),
  error: null
};

function appReducer(state, action) {
  switch (action.type) {
    case 'SET_STATUS': return { ...state, status: action.payload };
    case 'SET_QUERY': return { ...state, query: action.payload };
    case 'ADD_CHAT_MESSAGE': return { ...state, chatHistory: [...state.chatHistory, action.payload] };
    case 'START_GENERATING': return { ...state, status: 'GENERATING', currentAssistantMessage: '', references: [] };
    case 'APPEND_TOKEN': return { ...state, currentAssistantMessage: state.currentAssistantMessage + action.payload };
    case 'GENERATING_DONE': return { ...state, status: 'IDLE', chatHistory: [...state.chatHistory, { role: 'assistant', content: state.currentAssistantMessage, references: state.references }], currentAssistantMessage: '', references: [] };
    case 'SET_REFERENCES': return { ...state, references: action.payload };
    case 'SET_INDEXING_STATUS': return { ...state, indexingStatus: action.payload };
    case 'SET_ERROR': return { ...state, status: 'ERROR', error: action.payload };
    case 'RESET_CHAT': return { ...state, chatHistory: [], currentAssistantMessage: '', references: [], status: 'IDLE', selectedSourceIds: new Set() };
    case 'TOGGLE_SOURCE':
      const nextSelected = new Set(state.selectedSourceIds);
      if (nextSelected.has(action.payload)) nextSelected.delete(action.payload);
      else nextSelected.add(action.payload);
      return { ...state, selectedSourceIds: nextSelected };
    case 'CLEAR_SOURCES':
      return { ...state, selectedSourceIds: new Set() };
    default: return state;
  }
}

function App() {
  const [state, dispatch] = useReducer(appReducer, initialState);
  const [lang, setLang] = useState('zh');
  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState(false);
  const [isApiOk, setIsApiOk] = useState(null);
  const chatEndRef = useRef(null);
  const scrollContainerRef = useRef(null);
  const sendingRef = useRef(false);
  const appWindow = getCurrentWindow();

  useEffect(() => { if (state.status === 'GENERATING') chatEndRef.current?.scrollIntoView({ behavior: 'auto' }); }, [state.currentAssistantMessage]);

  useEffect(() => {
    let unlistens = [];
    let isMounted = true;
    async function setup() {
      try {
        const u1 = await listen('chat-token', (event) => { if (isMounted) dispatch({ type: 'APPEND_TOKEN', payload: event.payload }); });
        const u2 = await listen('chat-done', () => { if (isMounted) { sendingRef.current = false; dispatch({ type: 'GENERATING_DONE' }); } });
        const u3 = await listen('chat-error', (event) => { if (isMounted) { sendingRef.current = false; dispatch({ type: 'SET_ERROR', payload: event.payload }); } });
        const u4 = await listen('indexing-finished', () => { if (isMounted) invoke('get_indexing_status').then(res => dispatch({ type: 'SET_INDEXING_STATUS', payload: res })); });
        if (!isMounted) { u1(); u2(); u3(); u4(); } else { unlistens.push(u1, u2, u3, u4); }
      } catch (err) {}
    }
    setup();
    invoke('get_indexing_status').then(res => dispatch({ type: 'SET_INDEXING_STATUS', payload: res }));
    invoke('get_api_key').then(() => setIsApiOk(true)).catch(() => setIsApiOk(false));
    return () => { isMounted = false; unlistens.forEach(fn => fn()); };
  }, []);

  const handlePickFolder = async () => { try { await invoke('pick_folder'); } catch (err) { alert(err); } };

  const handleSend = async (customPrompt, displayPrompt) => {
    const finalQuery = customPrompt || state.query;
    if (!finalQuery.trim() || state.status === 'GENERATING' || sendingRef.current) return;
    sendingRef.current = true;
    dispatch({ type: 'SET_QUERY', payload: '' });
    dispatch({ type: 'ADD_CHAT_MESSAGE', payload: { role: 'user', content: displayPrompt || finalQuery } });
    dispatch({ type: 'START_GENERATING' });
    try {
      const selectedIdsArray = Array.from(state.selectedSourceIds);
      const refs = await invoke('ask_rag', { 
        query: finalQuery, 
        selectedIds: selectedIdsArray // 使用 camelCase 以适配 Tauri 默认映射
      });
      dispatch({ type: 'SET_REFERENCES', payload: refs });
    } catch (err) {
      sendingRef.current = false;
      console.error("RAG Error:", err);
      dispatch({ type: 'SET_ERROR', payload: err.toString() });
      // 注意：这里不再 SET_STATUS 为 IDLE，保留 ERROR 状态以供 UI 显示
    }
  };

  const handleQuickAction = (type) => {
    if (state.selectedSourceIds.size === 0) {
      alert("请先从检索结果中勾选至少一个研究素材。");
      return;
    }
    const prompts = {
      summary: ["请为选中的研究素材生成一份专业的研究综述，提取核心论点。", "📝 生成研究综述"],
      qa: ["请基于选中的素材提出 3 个最深刻的洞察问题并解答。", "❓ 深度问答"],
      table: ["请以 Markdown 表格形式提取素材中的关键数据（日期、金额、项目名等）。", "📊 数据提炼"]
    };
    handleSend(prompts[type][0], prompts[type][1]);
  };

  return (
    <div className="flex h-screen bg-[#121214] text-gray-300 font-sans overflow-hidden">
      <div onMouseDown={() => invoke('start_dragging')} className="absolute top-0 left-0 right-0 h-10 z-[100] cursor-grab flex items-center justify-between px-4">
        <div className="flex gap-2">
          <div onClick={() => appWindow.close()} className="w-3 h-3 rounded-full bg-red-500/20 hover:bg-red-500 cursor-pointer transition-colors" />
          <div onClick={() => appWindow.minimize()} className="w-3 h-3 rounded-full bg-yellow-500/20 hover:bg-yellow-500 cursor-pointer transition-colors" />
          <div onClick={async () => (await appWindow.isMaximized()) ? appWindow.unmaximize() : appWindow.maximize()} className="w-3 h-3 rounded-full bg-green-500/20 hover:bg-green-500 cursor-pointer transition-colors" />
        </div>
        <div className="flex items-center gap-2 text-[10px] text-neutral-500 tracking-wider font-medium"><Server size={10} className={isApiOk ? "text-green-500" : "text-red-500"} />{isApiOk ? "SERVICE READY" : "OFFLINE"}</div>
      </div>

      <div className="w-[260px] min-w-[220px] shrink-0 bg-[#0c0c0e] border-r border-neutral-900 flex flex-col pt-12">
        <div className="px-5 mb-8 flex justify-between items-center"><div className="flex items-center gap-2.5 text-white font-bold text-sm tracking-tight"><div className="p-1 bg-blue-600 rounded"><LayoutGrid size={14} className="text-white"/></div>VaultSeek</div><button onClick={() => setIsSettingsModalOpen(true)} className="p-1.5 text-neutral-500 hover:text-white hover:bg-neutral-800 rounded-md transition-all"><Settings size={14}/></button></div>
        <div className="px-3 flex-1 flex flex-col gap-1 overflow-y-auto">
          <button onClick={() => dispatch({ type: 'RESET_CHAT' })} className="flex items-center gap-3 px-3 py-2.5 text-sm text-neutral-400 hover:text-white hover:bg-neutral-800/50 rounded-xl transition-all border border-transparent hover:border-neutral-800"><MessageSquare size={16} /> 新对话</button>
          <div className="mt-8 px-3 text-[10px] font-bold text-neutral-600 tracking-widest uppercase flex items-center justify-between"><span>知识库状态</span><button onClick={handlePickFolder} className="hover:text-blue-500 transition-colors"><LayoutGrid size={10} /></button></div>
          {state.indexingStatus.watch_path ? (
            <div className="mt-2 px-3 py-3 rounded-xl bg-neutral-900/50 border border-neutral-800">
               <div className="flex items-center justify-between mb-2"><span className="text-[11px] text-neutral-400 truncate max-w-[120px]">{state.indexingStatus.watch_path.split('/').pop()}</span>{!state.indexingStatus.is_finished && <Loader2 size={10} className="animate-spin text-blue-500" />}</div>
               <div className="w-full h-1 bg-neutral-800 rounded-full overflow-hidden"><div className="h-full bg-blue-600 transition-all duration-500" style={{ width: `${state.indexingStatus.total > 0 ? (state.indexingStatus.current / state.indexingStatus.total * 100) : 100}%` }}/></div>
               <div className="mt-2 text-[10px] text-neutral-500">{state.indexingStatus.is_finished ? '索引已同步' : `正在同步 (${state.indexingStatus.current}/${state.indexingStatus.total})`}</div>
            </div>
          ) : <button onClick={handlePickFolder} className="mt-2 mx-1 flex items-center justify-center gap-2 py-3 border border-dashed border-neutral-800 rounded-xl text-xs text-neutral-500 hover:text-blue-400 hover:border-blue-900 transition-all"><FolderOpen size={14} /> 导入知识库</button>}
          
          {state.selectedSourceIds.size > 0 && (
            <div className="mt-8 px-3 animate-in fade-in duration-500">
               <div className="flex items-center justify-between border-b border-blue-500/20 pb-2 mb-3">
                 <div className="text-[10px] font-bold text-blue-500 uppercase tracking-widest italic">Researching ({state.selectedSourceIds.size})</div>
                 <button onClick={() => dispatch({ type: 'CLEAR_SOURCES' })} className="text-[9px] text-neutral-500 hover:text-red-400 transition-colors">CLEAR</button>
               </div>
               <div className="space-y-1">
                  {[...state.selectedSourceIds].map(id => (
                    <div key={id} className="text-[9px] text-neutral-500 truncate bg-blue-900/5 px-2 py-2 rounded-lg border border-blue-900/10 flex items-center gap-2 animate-in slide-in-from-left-2"><Zap size={8} className="text-blue-500" /> REF: {id.slice(-8)}</div>
                  ))}
               </div>
            </div>
          )}
        </div>
        <div className="p-4 border-t border-neutral-900"><div className="flex items-center gap-2 text-[10px] text-neutral-600"><Zap size={10} /> v1.2.0 Stable</div></div>
      </div>

      <div className="flex-1 flex flex-col bg-[#121214] min-w-[400px] relative">
        <div ref={scrollContainerRef} className="flex-1 overflow-y-auto px-6 py-10 pt-16">
          <div className="max-w-3xl mx-auto space-y-10">
            {state.chatHistory.length === 0 ? (
              <div className="flex flex-col items-center justify-center min-h-[60vh] text-center">
                <div className="w-16 h-16 bg-blue-600/10 rounded-3xl flex items-center justify-center mb-6 border border-blue-600/20"><Sparkles size={32} className="text-blue-500" /></div>
                <h1 className="text-2xl font-bold text-white mb-2 tracking-tight">有什么我可以帮您的？</h1>
                <p className="text-neutral-500 text-sm mb-10 max-w-sm">VaultSeek 已经准备好从您的本地知识库中提取智慧并进行总结。</p>
                <div className="grid grid-cols-2 gap-4 w-full max-w-lg">
                  <button onClick={() => handleSend("请总结当前知识库中最核心的观点，并列出关键数据。", "总结知识库核心")} className="p-4 bg-[#1c1c1f] hover:bg-[#27272a] rounded-2xl border border-neutral-800 text-left transition-all hover:scale-[1.02]"><div className="text-xs font-bold text-blue-400 mb-1">快速行动</div><div className="text-sm text-neutral-300">一键总结知识库观点</div></button>
                  <button onClick={() => handleSend("列出最近更新的文件中提到的核心概念。", "最近核心概念")} className="p-4 bg-[#1c1c1f] hover:bg-[#27272a] rounded-2xl border border-neutral-800 text-left transition-all hover:scale-[1.02]"><div className="text-xs font-bold text-indigo-400 mb-1">动态追踪</div><div className="text-sm text-neutral-300">分析最近变更的知识</div></button>
                </div>
              </div>
            ) : state.chatHistory.map((msg, i) => (
              <div key={i} className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                <ChatMessage msg={msg} isLast={i === state.chatHistory.length - 1} keywords={state.chatHistory[i-1]?.content} />
                {msg.references && <ReferenceList references={msg.references} query={state.chatHistory[i-1]?.content} selectedIds={state.selectedSourceIds} onToggle={(id) => dispatch({ type: 'TOGGLE_SOURCE', payload: id })} />}
              </div>
            ))}
            {state.status === 'GENERATING' && (
              <div className="animate-in fade-in duration-300">
                {!state.currentAssistantMessage && <ThinkingChain />}
                <ChatMessage msg={{ role: 'assistant', content: state.currentAssistantMessage }} keywords={state.chatHistory[state.chatHistory.length-1]?.content} />
                {state.references.length > 0 && <ReferenceList references={state.references} query={state.chatHistory[state.chatHistory.length-1]?.content} selectedIds={state.selectedSourceIds} onToggle={(id) => dispatch({ type: 'TOGGLE_SOURCE', payload: id })} />}
              </div>
            )}
            <div ref={chatEndRef} className="h-32" />
          </div>
        </div>
<div className="absolute bottom-0 left-0 right-0 p-8 pt-0 pointer-events-none">
  <div className="max-w-3xl mx-auto flex flex-col gap-4">
    {/* 错误提示显示区域 */}
    {state.status === 'ERROR' && (
      <div className="pointer-events-auto bg-red-500/10 border border-red-500/20 rounded-xl px-4 py-2 text-[10px] text-red-400 animate-in fade-in slide-in-from-bottom-2">
         ⚠️ 异常反馈: {state.error}
      </div>
    )}

    {/* Quick Actions Bar */}
    <div className="flex gap-2 px-1 pointer-events-auto overflow-x-auto scrollbar-hide">
               <button onClick={() => handleQuickAction('summary')} className="group flex items-center gap-2 px-4 py-2 rounded-xl bg-[#1c1c1f] border border-neutral-800 hover:border-blue-500/50 hover:bg-blue-600/5 text-xs text-neutral-400 hover:text-blue-400 transition-all shadow-xl">
                  <div className="w-5 h-5 rounded-lg bg-blue-500/10 flex items-center justify-center group-hover:bg-blue-500/20"><Zap size={10} /></div>
                  生成综述
               </button>
               <button onClick={() => handleQuickAction('qa')} className="group flex items-center gap-2 px-4 py-2 rounded-xl bg-[#1c1c1f] border border-neutral-800 hover:border-indigo-500/50 hover:bg-indigo-600/5 text-xs text-neutral-400 hover:text-indigo-400 transition-all shadow-xl">
                  <div className="w-5 h-5 rounded-lg bg-indigo-500/10 flex items-center justify-center group-hover:bg-indigo-500/20"><MessageSquare size={10} /></div>
                  深度问答
               </button>
               <button onClick={() => handleQuickAction('table')} className="group flex items-center gap-2 px-4 py-2 rounded-xl bg-[#1c1c1f] border border-neutral-800 hover:border-emerald-500/50 hover:bg-emerald-600/5 text-xs text-neutral-400 hover:text-emerald-400 transition-all shadow-xl">
                  <div className="w-5 h-5 rounded-lg bg-emerald-500/10 flex items-center justify-center group-hover:bg-emerald-500/20"><LayoutGrid size={10} /></div>
                  提炼表格
               </button>
            </div>

            <div className="glass-input rounded-2xl p-2 pointer-events-auto border border-white/5 shadow-2xl">
              <div className="relative flex items-end gap-2 p-2">
                <textarea 
                  value={state.query} 
                  onChange={(e) => dispatch({ type: 'SET_QUERY', payload: e.target.value })} 
                  onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && (e.preventDefault(), handleSend())} 
                  className="flex-1 bg-transparent border-none focus:ring-0 p-2 text-sm text-white placeholder-neutral-500 resize-none min-h-[44px] max-h-32 scrollbar-hide" 
                  rows={1} 
                  placeholder={
                    state.status === 'GENERATING' ? '分析引擎运行中...' : 
                    state.selectedSourceIds.size > 0 ? `当前处于【定向研读】模式 (仅在 ${state.selectedSourceIds.size} 个锁定素材中检索)...` : 
                    '询问您的全库私有知识...'
                  } 
                  disabled={state.status === 'GENERATING'} 
                />
                <button onClick={() => handleSend()} disabled={state.status === 'GENERATING' || !state.query.trim()} className="p-2.5 bg-blue-600 hover:bg-blue-500 disabled:bg-neutral-800 disabled:text-neutral-600 text-white rounded-xl transition-all shadow-lg shadow-blue-900/20">{state.status === 'GENERATING' ? <Loader2 size={18} className="animate-spin" /> : <Send size={18} />}</button>
              </div>
            </div>
          </div>
          <div className="max-w-3xl mx-auto mt-3 flex justify-center"><div className="flex items-center gap-4 text-[9px] text-neutral-600 font-bold tracking-widest uppercase"><span>VaultSeek Protocol</span><span className="w-1 h-1 bg-neutral-800 rounded-full" /><span>End-to-End Private</span><span className="w-1 h-1 bg-neutral-800 rounded-full" /><span>v1.2.0-S</span></div></div>
        </div>
      </div>
      <SettingsModal isOpen={isSettingsModalOpen} onClose={() => setIsSettingsModalOpen(false)} lang={lang} setLang={setLang} />
    </div>
  );
}

export default App;
