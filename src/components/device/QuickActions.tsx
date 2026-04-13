// Quick Actions Panel - Key event buttons for device control
import { motion } from 'framer-motion';
import {
    Home, Square, Power, Volume2, Volume1, VolumeX,
    Menu, Sun, Moon, Bell, Play, Triangle, Zap
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { listItem } from '../../lib/animations';
import { useLanguage } from '../../contexts/LanguageContext';

interface QuickActionsProps {
    deviceId: string;
}

export function QuickActions({ deviceId }: QuickActionsProps) {
    const { t } = useLanguage();

    const handleKeyEvent = async (keycode: number) => {
        try {
            await invoke('execute_shell', {
                deviceId,
                command: `input keyevent ${keycode}`
            });
        } catch (error) {
            console.error('Key event failed:', error);
            toast.error(t.failedToKey);
        }
    };

    return (
        <motion.div variants={listItem} className="bg-surface-card border border-border rounded-xl p-4">
            <h4 className="text-sm font-semibold text-text-primary mb-4 flex items-center gap-2">
                <Zap size={16} className="text-accent" />
                {t.quickActions}
            </h4>
            <div className="grid grid-cols-4 gap-2">
                {/* Row 1: Navigation */}
                <button onClick={() => handleKeyEvent(4)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.back}>
                    <Triangle size={18} className="text-text-secondary -rotate-90" />
                </button>
                <button onClick={() => handleKeyEvent(3)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.home}>
                    <Home size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(187)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.recents}>
                    <Square size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(82)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.menu}>
                    <Menu size={18} className="text-text-secondary" />
                </button>

                {/* Row 2: Volume & Power */}
                <button onClick={() => handleKeyEvent(25)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.volumeDown}>
                    <Volume1 size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(24)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.volumeUp}>
                    <Volume2 size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(164)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.mute}>
                    <VolumeX size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(26)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.power}>
                    <Power size={18} className="text-error" />
                </button>

                {/* Row 3: System & Media */}
                <button onClick={() => handleKeyEvent(220)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.brightnessDown}>
                    <Moon size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(221)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.brightnessUp}>
                    <Sun size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(83)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.notifications}>
                    <Bell size={18} className="text-text-secondary" />
                </button>
                <button onClick={() => handleKeyEvent(85)} className="p-2 bg-surface-elevated hover:bg-surface-hover border border-border rounded-lg flex items-center justify-center transition-colors" title={t.playPause}>
                    <Play size={18} className="text-success" />
                </button>
            </div>
        </motion.div>
    );
}
