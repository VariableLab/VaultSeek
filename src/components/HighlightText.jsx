// --- 辅助函数：正则表达式转义 ---
const escapeRegExp = (string) => {
  return string.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
};

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
          uniqueWords.some(w => part.toLowerCase().includes(w.toLowerCase())) ? (
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

export default HighlightText;
