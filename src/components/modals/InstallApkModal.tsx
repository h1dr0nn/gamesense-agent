import { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { Package, X, FolderOpen, FileCheck } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'sonner';
import type { DeviceInfo, ApkInfo } from '../../types';

interface InstallApkModalProps {
  device: DeviceInfo;
  onClose: () => void;
}

interface TauriDropPayload {
  paths: string[];
}

export function InstallApkModal({ device, onClose }: InstallApkModalProps) {
  const [apkPath, setApkPath] = useState('');
  const [apkInfo, setApkInfo] = useState<ApkInfo | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    const unHover = listen('tauri://drag-over', () => setIsDragging(true));
    const unDrop = listen<TauriDropPayload>('tauri://drag-drop', (e) => {
      setIsDragging(false);
      const path = e.payload.paths?.[0];
      if (path?.toLowerCase().endsWith('.apk')) handlePath(path);
    });
    const unCancel = listen('tauri://drag-cancelled', () => setIsDragging(false));
    const unLeave = listen('tauri://drag-leave', () => setIsDragging(false));
    return () => {
      unHover.then(fn => fn());
      unDrop.then(fn => fn());
      unCancel.then(fn => fn());
      unLeave.then(fn => fn());
    };
  }, []);

  const handlePath = async (path: string) => {
    setApkPath(path);
    try {
      const info = await invoke<ApkInfo>('validate_apk', { path });
      setApkInfo(info);
    } catch {
      setApkInfo({ path, file_name: path.split(/[\\/]/).pop() ?? path, size_bytes: 0, valid: false });
    }
  };

  const handleBrowse = async () => {
    const selected = await open({ multiple: false, filters: [{ name: 'APK', extensions: ['apk'] }] });
    if (selected && typeof selected === 'string') handlePath(selected);
  };

  const handleInstall = async () => {
    if (!apkPath) return;
    setInstalling(true);
    try {
      await invoke('install_apk', { deviceId: device.id, apkPath });
      toast.success(`${apkInfo?.file_name ?? 'APK'} installed successfully`);
      onClose();
    } catch (err) {
      toast.error(`Install failed: ${err}`);
    } finally {
      setInstalling(false);
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  return createPortal(
    <AnimatePresence>
      <motion.div
        className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        onClick={onClose}
      >
        <motion.div
          className="w-full max-w-md bg-surface-card border border-border rounded-2xl shadow-2xl"
          initial={{ scale: 0.95, opacity: 0, y: 8 }}
          animate={{ scale: 1, opacity: 1, y: 0 }}
          exit={{ scale: 0.95, opacity: 0, y: 8 }}
          transition={{ type: 'spring', stiffness: 400, damping: 30 }}
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="flex items-center justify-between px-5 py-4 border-b border-border">
            <div className="flex items-center gap-3">
              <div className="w-8 h-8 rounded-lg bg-accent/10 flex items-center justify-center">
                <Package size={16} className="text-accent" />
              </div>
              <div>
                <h3 className="text-sm font-semibold text-text-primary">Install APK</h3>
                <p className="text-xs text-text-muted">{device.model || device.id}</p>
              </div>
            </div>
            <button onClick={onClose} className="p-1.5 rounded-lg text-text-muted hover:text-text-primary hover:bg-surface-elevated transition-colors">
              <X size={16} />
            </button>
          </div>

          {/* Body */}
          <div className="p-5">
            {apkInfo ? (
              /* Selected APK info */
              <div className="flex items-center gap-3 p-4 bg-surface-elevated border border-border rounded-xl">
                <Package size={28} className="text-accent shrink-0" />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-text-primary truncate">{apkInfo.file_name}</p>
                  <div className="flex items-center gap-2 mt-0.5">
                    <span className="text-xs text-text-muted font-mono">{formatSize(apkInfo.size_bytes)}</span>
                    {apkInfo.valid
                      ? <span className="flex items-center gap-1 text-xs text-success"><FileCheck size={11} /> Valid</span>
                      : <span className="text-xs text-error">Invalid APK</span>
                    }
                  </div>
                </div>
                <button onClick={() => { setApkPath(''); setApkInfo(null); }} className="p-1.5 rounded-md text-text-muted hover:text-error hover:bg-error/10 transition-colors">
                  <X size={14} />
                </button>
              </div>
            ) : (
              /* Drop zone */
              <button
                onClick={handleBrowse}
                className={`w-full border-2 border-dashed rounded-xl p-10 flex flex-col items-center gap-3 transition-all ${
                  isDragging ? 'border-accent bg-accent/10' : 'border-border hover:border-accent hover:bg-accent/5'
                }`}
              >
                <Package size={40} className={isDragging ? 'text-accent' : 'text-text-muted opacity-40'} />
                <div className="text-center">
                  <p className="text-sm font-medium text-text-primary">Drop APK here</p>
                  <p className="text-xs text-text-muted mt-1 flex items-center justify-center gap-1">
                    <FolderOpen size={11} /> or click to browse
                  </p>
                </div>
              </button>
            )}
          </div>

          {/* Footer */}
          <div className="flex items-center justify-end gap-2 px-5 py-4 border-t border-border bg-surface-elevated/40 rounded-b-2xl">
            <button
              onClick={onClose}
              className="px-4 py-2 rounded-xl text-sm font-medium text-text-secondary hover:text-text-primary border border-border hover:bg-surface-elevated transition-all"
            >
              Cancel
            </button>
            <button
              onClick={handleInstall}
              disabled={!apkInfo?.valid || installing}
              className="px-4 py-2 rounded-xl text-sm font-medium bg-accent text-white hover:bg-accent/90 transition-all active:scale-95 disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2"
            >
              {installing
                ? <><span className="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin" />Installing...</>
                : <><Package size={14} />Install</>
              }
            </button>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>,
    document.body
  );
}
