import React, { useReducer, useEffect, useRef, useState } from 'react';
import { FolderOpen, Settings, Send, Loader2, Sparkles, LayoutGrid, Zap, Server, MessageSquare, Brain } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import SettingsModal from './components/SettingsModal';
import ChatMessage from './components/ChatMessage';
import ReferenceList from './components/ReferenceList';
import ThinkingChain from './components/ThinkingChain';
import { initialState, appReducer } from './store/appReducer';
import { useTranslation } from 'react-i18next';

function App() {
  const { t } = useTranslation();
  const [state, dispatch] = useReducer(appReducer, initialState);
  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState(false);
  const [isApiOk, setIsApiOk] = useState(null);
  const chatEndRef = useRef(null);
  const scrollContainerRef = useRef(null);
  const sendingRef = useRef(false);
  const appWindow = useRef(getCurrentWindow()).current;

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
      } catch (err) { console.error('Failed to setup event listeners:', err); }
    }
    setup();
    invoke('get_indexing_status').then(res => dispatch({ type: 'SET_INDEXING_STATUS', payload: res }));
    invoke('check_api_key_status').then((isSet) => setIsApiOk(isSet)).catch(() => setIsApiOk(false));
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
        selectedIds: selectedIdsArray,
        persona: state.persona
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
      summary: [t('prompt_summary'), t('action_summary')],
      qa: [t('prompt_qa'), t('action_qa')],
      table: [t('prompt_table'), t('action_table')]
    };
    handleSend(prompts[type][0], prompts[type][1]);
  };

  return (
    <div className="flex h-screen bg-[#121214] text-gray-300 font-sans overflow-hidden">
      <div onMouseDown={() => invoke('start_dragging')} className="absolute top-0 left-0 right-0 h-10 z-[100] cursor-grab flex items-center justify-between px-4">
        <div className="flex gap-2">
          <div onClick={() => invoke('close_window')} className="w-3 h-3 rounded-full bg-red-500/20 hover:bg-red-500 cursor-pointer transition-colors" />
          <div onClick={() => invoke('minimize_window')} className="w-3 h-3 rounded-full bg-yellow-500/20 hover:bg-yellow-500 cursor-pointer transition-colors" />
          <div onClick={() => invoke('maximize_window')} className="w-3 h-3 rounded-full bg-green-500/20 hover:bg-green-500 cursor-pointer transition-colors" />
        </div>
        <div className="flex items-center gap-2 text-[10px] text-neutral-500 tracking-wider font-medium"><Server size={10} className={isApiOk ? "text-green-500" : "text-red-500"} />{isApiOk ? "SERVICE READY" : "OFFLINE"}</div>
      </div>

      <div className="w-[260px] min-w-[220px] shrink-0 bg-[#0c0c0e] border-r border-neutral-900 flex flex-col pt-12">
        <div className="px-5 mb-8 flex justify-between items-center"><div className="flex items-center gap-2.5 text-white font-bold text-sm tracking-tight"><div className="p-1 bg-blue-600 rounded"><LayoutGrid size={14} className="text-white"/></div>{t('app_title')}</div><button onClick={() => setIsSettingsModalOpen(true)} className="p-1.5 text-neutral-500 hover:text-white hover:bg-neutral-800 rounded-md transition-all"><Settings size={14}/></button></div>
        <div className="px-3 flex-1 flex flex-col gap-1 overflow-y-auto">
          <button onClick={() => dispatch({ type: 'RESET_CHAT' })} className="flex items-center gap-3 px-3 py-2.5 text-sm text-neutral-400 hover:text-white hover:bg-neutral-800/50 rounded-xl transition-all border border-transparent hover:border-neutral-800"><MessageSquare size={16} /> {t('new_chat')}</button>
          
          <div className="mt-4 px-3">
             <div className="text-[10px] font-bold text-neutral-600 tracking-widest uppercase mb-2 flex items-center gap-2"><Brain size={10}/> {t('expert_persona')}</div>
             <select 
                value={state.persona} 
                onChange={(e) => dispatch({ type: 'SET_PERSONA', payload: e.target.value })}
                className="w-full bg-[#1c1c1f] text-xs text-neutral-300 border border-neutral-800 rounded-lg p-2 focus:ring-0 focus:border-blue-500 transition-colors cursor-pointer outline-none"
             >
                <option value="default">🎭 {t('persona_normal')}</option>
                <option value="medical">🩺 {t('persona_medical')}</option>
                <option value="legal">⚖️ {t('persona_legal')}</option>
                <option value="coder">💻 {t('persona_coder')}</option>
             </select>
          </div>

          <div className="mt-8 px-3 text-[10px] font-bold text-neutral-600 tracking-widest uppercase flex items-center justify-between"><span>{t('kb_status')}</span><button onClick={handlePickFolder} className="hover:text-blue-500 transition-colors"><LayoutGrid size={10} /></button></div>
          {state.indexingStatus.watch_path ? (
            <div className="mt-2 px-3 py-3 rounded-xl bg-neutral-900/50 border border-neutral-800">
               <div className="flex items-center justify-between mb-2"><span className="text-[11px] text-neutral-400 truncate max-w-[120px]">{state.indexingStatus.watch_path.split('/').pop()}</span>{!state.indexingStatus.is_finished && <Loader2 size={10} className="animate-spin text-blue-500" />}</div>
               <div className="w-full h-1 bg-neutral-800 rounded-full overflow-hidden"><div className="h-full bg-blue-600 transition-all duration-500" style={{ width: `${state.indexingStatus.total > 0 ? (state.indexingStatus.current / state.indexingStatus.total * 100) : 100}%` }}/></div>
               <div className="mt-2 text-[10px] text-neutral-500">{state.indexingStatus.is_finished ? t('status_idle') : `${t('status_indexing')} (${state.indexingStatus.current}/${state.indexingStatus.total})`}</div>
            </div>
          ) : <button onClick={handlePickFolder} className="mt-2 mx-1 flex items-center justify-center gap-2 py-3 border border-dashed border-neutral-800 rounded-xl text-xs text-neutral-500 hover:text-blue-400 hover:border-blue-900 transition-all"><FolderOpen size={14} /> {t('import_kb')}</button>}
          
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
                <h1 className="text-2xl font-bold text-white mb-2 tracking-tight">{t('welcome_title')}</h1>
                <p className="text-neutral-500 text-sm mb-10 max-w-sm">{t('welcome_desc')}</p>
                <div className="grid grid-cols-2 gap-4 w-full max-w-lg">
                  <button onClick={() => handleSend(t('prompt_summary'), t('action_summary'))} className="p-4 bg-[#1c1c1f] hover:bg-[#27272a] rounded-2xl border border-neutral-800 text-left transition-all hover:scale-[1.02]"><div className="text-xs font-bold text-blue-400 mb-1">{t('quick_action')}</div><div className="text-sm text-neutral-300">{t('quick_action_desc')}</div></button>
                  <button onClick={() => handleSend("List the core concepts mentioned in recently updated files.", t('dynamic_track'))} className="p-4 bg-[#1c1c1f] hover:bg-[#27272a] rounded-2xl border border-neutral-800 text-left transition-all hover:scale-[1.02]"><div className="text-xs font-bold text-indigo-400 mb-1">{t('dynamic_track')}</div><div className="text-sm text-neutral-300">{t('dynamic_track_desc')}</div></button>
                </div>
              </div>
            ) : state.chatHistory.map((msg, i) => (
              <div key={i} className="animate-in fade-in slide-in-from-bottom-2 duration-300">
                <ChatMessage msg={msg} isLast={i === state.chatHistory.length - 1} keywords={state.chatHistory[i-1]?.content} />
              </div>
            ))}
            {state.status === 'GENERATING' && (
              <div className="animate-in fade-in duration-300">
                {!state.currentAssistantMessage && <ThinkingChain />}
                <ChatMessage msg={{ role: 'assistant', content: state.currentAssistantMessage }} keywords={state.chatHistory[state.chatHistory.length-1]?.content} />
              </div>
            )}
            <div ref={chatEndRef} className="h-32" />
          </div>
        </div>
<div className="absolute bottom-0 left-0 right-0 p-8 pt-0 pointer-events-none">
  <div className="max-w-3xl mx-auto flex flex-col gap-4">
    {/* 错误提示显示区域 */}
    {state.status === 'ERROR' && (
      <div className="pointer-events-auto bg-red-500/10 border border-red-500/20 rounded-xl px-4 py-2 text-[10px] text-red-400 animate-in fade-in slide-in-from-bottom-2 flex items-center justify-between">
         <span>⚠️ 异常反馈: {state.error}</span>
         <button onClick={() => dispatch({ type: 'CLEAR_ERROR' })} className="ml-4 px-2 py-0.5 rounded bg-red-500/20 hover:bg-red-500/40 text-red-300 text-[10px] transition-colors shrink-0">关闭</button>
      </div>
    )}

    {/* Quick Actions Bar */}
    <div className="flex gap-2 px-1 pointer-events-auto overflow-x-auto scrollbar-hide">
               <button onClick={() => handleQuickAction('summary')} className="group flex items-center gap-2 px-4 py-2 rounded-xl bg-[#1c1c1f] border border-neutral-800 hover:border-blue-500/50 hover:bg-blue-600/5 text-xs text-neutral-400 hover:text-blue-400 transition-all shadow-xl">
                  <div className="w-5 h-5 rounded-lg bg-blue-500/10 flex items-center justify-center group-hover:bg-blue-500/20"><Zap size={10} /></div>
                  {t('action_summary')}
               </button>
               <button onClick={() => handleQuickAction('qa')} className="group flex items-center gap-2 px-4 py-2 rounded-xl bg-[#1c1c1f] border border-neutral-800 hover:border-indigo-500/50 hover:bg-indigo-600/5 text-xs text-neutral-400 hover:text-indigo-400 transition-all shadow-xl">
                  <div className="w-5 h-5 rounded-lg bg-indigo-500/10 flex items-center justify-center group-hover:bg-indigo-500/20"><MessageSquare size={10} /></div>
                  {t('action_qa')}
               </button>
               <button onClick={() => handleQuickAction('table')} className="group flex items-center gap-2 px-4 py-2 rounded-xl bg-[#1c1c1f] border border-neutral-800 hover:border-emerald-500/50 hover:bg-emerald-600/5 text-xs text-neutral-400 hover:text-emerald-400 transition-all shadow-xl">
                  <div className="w-5 h-5 rounded-lg bg-emerald-500/10 flex items-center justify-center group-hover:bg-emerald-500/20"><LayoutGrid size={10} /></div>
                  {t('action_table')}
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
                    state.status === 'GENERATING' ? t('status_indexing') : 
                    state.selectedSourceIds.size > 0 ? `${t('searching_locked')} (仅在 ${state.selectedSourceIds.size} 个锁定素材中检索)...` : 
                    t('placeholder_empty')
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

      {/* Right Sidebar: Evidence Vault */}
      <div className="w-[320px] min-w-[280px] shrink-0 bg-[#0c0c0e] border-l border-neutral-900 flex flex-col relative z-40">
         <div className="px-5 h-12 border-b border-neutral-900 flex items-center gap-2 text-neutral-400 text-xs font-bold tracking-widest uppercase pt-6">
            <Zap size={12} className="text-blue-500"/> {t('evidence_vault')}
         </div>
         <div className="flex-1 overflow-y-auto p-4 custom-scrollbar">
            {state.status === 'IDLE' && state.chatHistory.length === 0 ? (
               <div className="flex flex-col items-center justify-center h-full text-center px-4 opacity-50">
                  <LayoutGrid size={24} className="text-neutral-600 mb-2" />
                  <p className="text-[10px] text-neutral-500">{t('no_sources_yet')}</p>
               </div>
            ) : (
               <ReferenceList 
                  references={state.references} 
                  query={state.query || state.chatHistory[state.chatHistory.length - 2]?.content} 
                  selectedIds={state.selectedSourceIds} 
                  onToggle={(id) => dispatch({ type: 'TOGGLE_SOURCE', payload: id })} 
               />
            )}
         </div>
      </div>

      <SettingsModal isOpen={isSettingsModalOpen} onClose={() => setIsSettingsModalOpen(false)} />
    </div>
  );
}

export default App;
