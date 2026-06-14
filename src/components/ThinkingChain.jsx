import { useState, useEffect } from 'react';
import { Loader2 } from 'lucide-react';

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

export default ThinkingChain;
