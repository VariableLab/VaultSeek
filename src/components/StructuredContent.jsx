import HighlightText from './HighlightText';

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

export default StructuredContent;
