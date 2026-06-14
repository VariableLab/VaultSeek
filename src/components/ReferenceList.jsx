import { memo } from 'react';
import { Zap, ExternalLink } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import HighlightText from './HighlightText';

const ReferenceList = memo(({ references, query, selectedIds, onToggle }) => {
  if (!references || references.length === 0) return null;
  return (
    <div className="space-y-4">
      <div className="text-[10px] font-black text-neutral-600 uppercase tracking-[0.3em] flex items-center gap-4 mb-2">
         <div className="h-[1px] w-4 bg-neutral-800" />
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

export default ReferenceList;
