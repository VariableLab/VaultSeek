import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';

export default function SettingsModal({ isOpen, onClose }) {
  const { t, i18n } = useTranslation();
  const [apiKey, setApiKey] = useState('');
  const [apiKeyPlaceholder, setApiKeyPlaceholder] = useState('未设置');
  const [apiUrl, setApiUrl] = useState('https://deepstock.zone.id/v1/chat/completions');
  const [model, setModel] = useState('moonshotai/kimi-k2.6');

  useEffect(() => {
    if (isOpen) {
      invoke('check_api_key_status').then(isSet => {
         setApiKeyPlaceholder(isSet ? '•••••••••••••••• (Secured)' : 'Not Set');
      }).catch(() => setApiKeyPlaceholder('Not Set'));
      
      invoke('get_setting', { key: 'api_url' }).then(setApiUrl).catch(() => {});
      invoke('get_setting', { key: 'model' }).then(setModel).catch(() => {});
      
      // Load saved language
      invoke('get_setting', { key: 'language' }).then(l => {
        if (l && l !== i18n.language) {
          i18n.changeLanguage(l);
        }
      }).catch(() => {});
    }
  }, [isOpen]);

  const handleSave = async () => {
    try {
      if (apiKey.trim() !== '') {
        await invoke('save_api_key', { key: apiKey });
      }
      await invoke('save_setting', { key: 'api_url', value: apiUrl });
      await invoke('save_setting', { key: 'model', value: model });
      await invoke('save_setting', { key: 'language', value: i18n.language });
      onClose();
    } catch (e) {
      alert(t('cancel') + " Failed: " + e);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-[200] flex items-center justify-center">
      <div className="bg-[#18181b] p-6 rounded-xl border border-neutral-700 w-[400px]">
        <h2 className="text-lg font-bold mb-4">{t('settings')}</h2>
        <div className="space-y-4">
          <div>
            <label className="text-xs text-neutral-400">{t('api_key')}</label>
            <input 
              type="password" 
              placeholder={apiKeyPlaceholder}
              value={apiKey} 
              onChange={(e) => setApiKey(e.target.value)} 
              className="w-full bg-[#2d2d2d] border border-neutral-700 rounded-lg p-2 text-sm text-white placeholder-neutral-500 focus:outline-none focus:border-blue-500" 
            />
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
            <label className="text-xs text-neutral-400">{t('language')}</label>
            <select value={i18n.language} onChange={(e) => i18n.changeLanguage(e.target.value)} className="w-full bg-[#2d2d2d] border border-neutral-700 rounded-lg p-2 text-sm text-white outline-none">
              <option value="zh">{t('lang_zh')}</option>
              <option value="en">{t('lang_en')}</option>
            </select>
          </div>
        </div>
        <div className="flex justify-end gap-2 mt-6">
          <button onClick={onClose} className="px-4 py-2 bg-neutral-700 hover:bg-neutral-600 rounded-lg text-sm text-white transition-colors">{t('cancel')}</button>
          <button onClick={handleSave} className="px-4 py-2 bg-blue-600 hover:bg-blue-500 rounded-lg text-sm text-white transition-colors">{t('save')}</button>
        </div>
      </div>
    </div>
  );
}
