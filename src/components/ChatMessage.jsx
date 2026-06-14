import { memo } from 'react';
import { Bot } from 'lucide-react';
import HighlightText from './HighlightText';
import StructuredContent from './StructuredContent';

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

export default ChatMessage;
