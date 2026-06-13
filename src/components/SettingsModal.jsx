import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { i18n } from '../i18n';

export default function SettingsModal({ isOpen, onClose, lang, setLang }) {
  const [apiKey, setApiKey] = useState('');
  const [apiUrl, setApiUrl] = useState('https://deepstock.zone.id/v1/chat/completions');
  const [model, setModel] = useState('moonshotai/kimi-k2.6');

  useEffect(() => {
    if (isOpen) {
      invoke('get_api_key').then(setApiKey).catch(() => setApiKey(''));
      invoke('get_setting', { key: 'api_url' }).then(setApiUrl).catch(() => {});
      invoke('get_setting', { key: 'model' }).then(setModel).catch(() => {});
    }
  }, [isOpen]);

  const handleSave = async () => {
    try {
      if (apiKey.trim() !== '') {
        await invoke('save_api_key', { key: apiKey });
      }
      await invoke('save_setting', { key: 'api_url', value: apiUrl });
      await invoke('save_setting', { key: 'model', value: model });
      await invoke('save_setting', { key: 'language', value: lang });
      onClose();
    } catch (e) {
      alert("保存设置失败: " + e);
    }
  };

  if (!isOpen) return null;

  const t = i18n[lang];

  return (
    <div className="fixed inset-0 bg-black/50 z-[200] flex items-center justify-center">
      <div className="bg-[#18181b] p-6 rounded-xl border border-neutral-700 w-[400px]">
        <h2 className="text-lg font-bold mb-4">{t.settings}</h2>
        <div className="space-y-4">
          <div>
            <label className="text-xs text-neutral-400">{t.apiKey}</label>
            <input value={apiKey} onChange={(e) => setApiKey(e.target.value)} className="w-full bg-[#2d2d2d] border border-neutral-700 rounded-lg p-2 text-sm text-white" />
          </div>
          <div>
            <label className="text-xs text-neutral-400">API URL</label>
            <input value={apiUrl} onChange={(e) => setApiUrl(e.target.value)} className="w-full bg-[#2d2d2d] border border-neutral-700 rounded-lg p-2 text-sm text-white" />
          </div>
          <div>
            <label className="text-xs text-neutral-400">Model</label>
            <input value={model} onChange={(e) => setModel(e.target.value)} className="w-full bg-[#2d2d2d] border border-neutral-700 rounded-lg p-2 text-sm text-white" />
          </div>
          <div>
            <label className="text-xs text-neutral-400">{t.language}</label>
            <select value={lang} onChange={(e) => setLang(e.target.value)} className="w-full bg-[#2d2d2d] border border-neutral-700 rounded-lg p-2 text-sm text-white">
              <option value="zh">中文</option>
              <option value="en">English</option>
            </select>
          </div>
        </div>
        <div className="flex justify-end gap-2 mt-6">
          <button onClick={onClose} className="px-4 py-2 bg-neutral-700 rounded-lg text-sm">{t.cancel}</button>
          <button onClick={handleSave} className="px-4 py-2 bg-blue-600 rounded-lg text-sm">{t.save}</button>
        </div>
      </div>
    </div>
  );
}
