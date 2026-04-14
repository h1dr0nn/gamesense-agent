// Settings Component
import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
    ArrowLeft, Moon, Sun, Monitor, FolderOpen, Globe,
    FileText, Settings as SettingsIcon, RotateCcw, Github,
    Bot, Eye, EyeOff, CheckCircle2, XCircle, BookOpen
} from 'lucide-react';
import { toast } from 'sonner';
import { open as openDialog, confirm } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';
import { Select } from './ui/Select';
import { check } from '@tauri-apps/plugin-updater';
import { useLanguage } from '../contexts/LanguageContext';
import { useTheme } from '../contexts/ThemeContext';
import { useDevices } from '../hooks/useDevices';

interface SettingsProps {
    onBack: () => void;
}

type ThemeMode = 'light' | 'dark' | 'system';
type AiProvider = 'gemini' | 'openai' | 'ollama';

const AI_PROVIDERS = [
    { value: 'gemini', label: 'Google Gemini', icon: <span className="text-xs">✦</span> },
    { value: 'openai', label: 'OpenAI', icon: <span className="text-xs">⬡</span> },
    { value: 'ollama', label: 'Ollama (local)', icon: <span className="text-xs">◈</span> },
];


export function Settings({ onBack }: SettingsProps) {
    const [notifications, setNotifications] = useState(true);
    const [adbPath, setAdbPath] = useState('');
    const [captureSavePath, setCaptureSavePath] = useState('');
    const [askBeforeSave, setAskBeforeSave] = useState(false);
    const [aiProvider, setAiProvider] = useState<AiProvider>('gemini');
    const [geminiApiKey, setGeminiApiKey] = useState('');
    const [geminiModel, setGeminiModel] = useState('gemini-2.0-flash');
    const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434');
    const [showApiKey, setShowApiKey] = useState(false);
    const [useCustomEndpoint, setUseCustomEndpoint] = useState(false);
    const [customEndpoint, setCustomEndpoint] = useState('');
    const [vaultEnabled, setVaultEnabled] = useState(false);
    const [vaultPath, setVaultPath] = useState('');

    const { language, setLanguage, t } = useLanguage();
    const { theme, setTheme } = useTheme();
    const { adbStatus } = useDevices();

    const loadSettings = () => {
        const storedNotif = localStorage.getItem('notifications');
        const storedAdbPath = localStorage.getItem('adbPath');
        const storedCapturePath = localStorage.getItem('captureSavePath');
        const storedAskBeforeSave = localStorage.getItem('askBeforeSave');

        if (storedNotif) setNotifications(storedNotif === 'true');
        else setNotifications(true);

        if (storedAdbPath) setAdbPath(storedAdbPath);
        else setAdbPath('');

        if (storedAskBeforeSave) setAskBeforeSave(storedAskBeforeSave === 'true');
        else setAskBeforeSave(false);

        if (storedCapturePath) {
            setCaptureSavePath(storedCapturePath);
        } else {
            invoke<string>('get_default_media_dir').then(path => {
                setCaptureSavePath(path);
                localStorage.setItem('captureSavePath', path);
            }).catch(console.error);
        }

        const storedProvider = localStorage.getItem('ai_provider');
        if (storedProvider) setAiProvider(storedProvider as AiProvider);

        const storedKey = localStorage.getItem('gemini_api_key');
        if (storedKey) setGeminiApiKey(storedKey);

        const storedModel = localStorage.getItem('gemini_model');
        if (storedModel) setGeminiModel(storedModel);

        const storedOllamaUrl = localStorage.getItem('ollama_url');
        if (storedOllamaUrl) setOllamaUrl(storedOllamaUrl);

        const storedUseCustomEndpoint = localStorage.getItem('use_custom_endpoint');
        if (storedUseCustomEndpoint) setUseCustomEndpoint(storedUseCustomEndpoint === 'true');

        const storedCustomEndpoint = localStorage.getItem('custom_endpoint');
        if (storedCustomEndpoint) setCustomEndpoint(storedCustomEndpoint);

        const storedVaultEnabled = localStorage.getItem('vault_enabled');
        if (storedVaultEnabled) setVaultEnabled(storedVaultEnabled === 'true');
        const storedVaultPath = localStorage.getItem('vault_path');
        if (storedVaultPath) setVaultPath(storedVaultPath);
    };

    useEffect(() => {
        loadSettings();
    }, []);

    const languages = [
        { value: 'en', label: 'English', icon: <span className="text-xs">🇺🇸</span> },
        { value: 'vi', label: 'Tiếng Việt', icon: <span className="text-xs">🇻🇳</span> },
    ];

    const themes: { value: ThemeMode; icon: React.ReactNode; label: string }[] = [
        { value: 'light', icon: <Sun size={16} />, label: t.light },
        { value: 'dark', icon: <Moon size={16} />, label: t.dark },
        { value: 'system', icon: <Monitor size={16} />, label: t.system },
    ];

    const handleLanguageChange = (val: string) => {
        setLanguage(val as any);
    };

    const handleThemeChange = (val: ThemeMode) => {
        setTheme(val);
    };

    const handleNotificationToggle = () => {
        const newState = !notifications;
        setNotifications(newState);
        localStorage.setItem('notifications', String(newState));
        toast.success(`Notifications ${newState ? 'Enabled' : 'Disabled'}`);
    };

    const handleProviderChange = (provider: AiProvider) => {
        setAiProvider(provider);
        localStorage.setItem('ai_provider', provider);
    };

    const handleSaveApiKey = () => {
        localStorage.setItem('gemini_api_key', geminiApiKey);
        toast.success('API key saved');
    };

    const handleModelChange = (model: string) => {
        setGeminiModel(model);
        localStorage.setItem('gemini_model', model);
    };

    const handleBrowseAdb = async () => {
        toast.info("Browser unavailable in debug mode");
    };

    const handleBrowseCapturePath = async () => {
        try {
            const selected = await openDialog({
                directory: true,
                multiple: false,
                title: 'Select Capture Save Location',
            });

            if (selected && typeof selected === 'string') {
                setCaptureSavePath(selected);
                localStorage.setItem('captureSavePath', selected);
                toast.success('Capture save path updated');
            }
        } catch (err) {
            console.error('Failed to browse', err);
            toast.info("Browser feature not available");
        }
    };

    const handleAskBeforeSaveToggle = () => {
        const newState = !askBeforeSave;
        setAskBeforeSave(newState);
        localStorage.setItem('askBeforeSave', String(newState));
        toast.success(`Ask before save ${newState ? 'Enabled' : 'Disabled'}`);
    };

    const handleResetDefaults = async () => {
        const confirmed = await confirm(t.resetConfirm, {
            title: t.resetToDefaults,
            kind: 'warning',
        });

        if (confirmed) {
            localStorage.clear();
            setTheme('system');
            setLanguage('en');
            loadSettings();
            toast.success(t.resetSuccess);
        }
    };

    const handleVaultToggle = () => {
        const next = !vaultEnabled;
        setVaultEnabled(next);
        localStorage.setItem('vault_enabled', String(next));
    };

    const handleBrowseVault = async () => {
        try {
            const selected = await openDialog({
                directory: true,
                multiple: false,
                title: 'Select Obsidian Vault Root',
            });
            if (selected && typeof selected === 'string') {
                setVaultPath(selected);
                localStorage.setItem('vault_path', selected);
                toast.success('Vault path updated');
            }
        } catch (err) {
            toast.info('Browser feature not available');
        }
    };

    const handleViewLogs = async () => {
        toast.info("Logs unavailable in debug mode");
    };

    const handleCheckUpdates = async () => {
        try {
            const update = await check();
            if (update) {
                const confirmed = await confirm(
                    `New version ${update.version} is available! Would you like to update?`,
                    { title: 'Update Available', kind: 'info' }
                );
                if (confirmed) {
                    toast.info('Downloading update...');
                    await update.downloadAndInstall();
                    toast.success('Update installed! Please restart the app.');
                }
            } else {
                toast.success(t.latestVersion || 'You are on the latest version!');
            }
        } catch (err) {
            console.error('Failed to check for updates', err);
            toast.error('Failed to check for updates. Make sure you have an internet connection.');
        }
    };


    return (
        <motion.div
            className="flex flex-col h-full"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.2 }}
        >
            {/* Header */}
            <div className="flex items-center gap-4 mb-4 shrink-0 px-1">
                <button
                    onClick={onBack}
                    className="p-2.5 rounded-xl hover:bg-surface-elevated text-text-secondary hover:text-text-primary transition-all duration-200 border border-transparent hover:border-border"
                >
                    <ArrowLeft size={22} />
                </button>
                <div className="flex items-center gap-3">
                    <div className="w-10 h-10 rounded-xl bg-accent/10 flex items-center justify-center">
                        <SettingsIcon className="text-accent" size={20} />
                    </div>
                    <div>
                        <h2 className="text-xl font-bold text-text-primary leading-tight">{t.settings}</h2>
                        <p className="text-xs text-text-muted">{t.managePrefs}</p>
                    </div>
                </div>
            </div>

            <div className="flex-1 overflow-y-auto no-scrollbar pb-8 space-y-6 px-1">
                {/* AI Provider Section */}
                <section className="bg-surface-card border border-border rounded-2xl p-6 shadow-sm">
                    <h3 className="text-sm font-bold text-text-muted uppercase tracking-wider mb-5 flex items-center gap-2">
                        <Bot size={16} className="text-accent" />
                        AI Provider
                    </h3>
                    <div className="space-y-5">
                        {/* Provider selector */}
                        <div className="flex items-center justify-between gap-4">
                            <div>
                                <p className="text-sm font-semibold text-text-primary">Provider</p>
                                <p className="text-xs text-text-secondary mt-0.5">AI backend for game analysis</p>
                            </div>
                            <div className="w-44">
                                <Select
                                    options={AI_PROVIDERS}
                                    value={aiProvider}
                                    onChange={(v) => handleProviderChange(v as AiProvider)}
                                />
                            </div>
                        </div>

                        {/* Provider-specific fields */}
                        <div className="pt-5 border-t border-border/50 space-y-4">
                            <ProviderFields
                                provider={aiProvider}
                                apiKey={geminiApiKey}
                                onApiKeyChange={setGeminiApiKey}
                                onSaveApiKey={handleSaveApiKey}
                                showApiKey={showApiKey}
                                onToggleShow={() => setShowApiKey(!showApiKey)}
                                model={geminiModel}
                                onModelChange={handleModelChange}
                                ollamaUrl={ollamaUrl}
                                onOllamaUrlChange={(v) => { setOllamaUrl(v); localStorage.setItem('ollama_url', v); }}
                                useCustomEndpoint={useCustomEndpoint}
                                onToggleCustomEndpoint={() => {
                                    const next = !useCustomEndpoint;
                                    setUseCustomEndpoint(next);
                                    localStorage.setItem('use_custom_endpoint', String(next));
                                }}
                                customEndpoint={customEndpoint}
                                onCustomEndpointChange={(v) => { setCustomEndpoint(v); localStorage.setItem('custom_endpoint', v); }}
                            />
                        </div>

                        {/* ADB Status */}
                        <div className="pt-5 border-t border-border/50 flex items-center justify-between">
                            <div>
                                <p className="text-sm font-semibold text-text-primary">ADB Status</p>
                                <p className="text-xs text-text-secondary mt-0.5">Android Debug Bridge connection</p>
                            </div>
                            <div className={`flex items-center gap-2 px-3 py-1.5 rounded-full border text-xs font-medium ${
                                adbStatus?.available
                                    ? 'bg-success/10 border-success/20 text-success'
                                    : 'bg-error/10 border-error/20 text-error'
                            }`}>
                                <div className={`w-1.5 h-1.5 rounded-full ${
                                    adbStatus?.available ? 'bg-success' : 'bg-error'
                                }`} />
                                {adbStatus?.available ? (adbStatus.version || 'ADB Ready') : 'ADB Not Found'}
                            </div>
                        </div>
                    </div>
                </section>

                {/* Knowledge Base Section */}
                <section className="bg-surface-card border border-border rounded-2xl p-6 shadow-sm">
                    <h3 className="text-sm font-bold text-text-muted uppercase tracking-wider mb-5 flex items-center gap-2">
                        <BookOpen size={16} className="text-accent" />
                        Knowledge Base
                    </h3>
                    <div className="space-y-5">
                        {/* Enable toggle */}
                        <div className="flex items-center justify-between">
                            <div>
                                <p className="text-sm font-semibold text-text-primary">Obsidian Logging</p>
                                <p className="text-xs text-text-secondary mt-0.5">Log agent sessions to Obsidian vault</p>
                            </div>
                            <button
                                onClick={handleVaultToggle}
                                className={`relative w-11 h-6 rounded-full transition-colors duration-200 focus:outline-none flex items-center ${vaultEnabled ? 'bg-accent border border-accent' : 'bg-surface-elevated border border-border'}`}
                            >
                                <motion.div
                                    className="w-4 h-4 bg-white rounded-full shadow-sm ml-1"
                                    animate={{ x: vaultEnabled ? 18 : 0 }}
                                    transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                                />
                            </button>
                        </div>

                        {/* Vault path */}
                        <div className={`pt-5 border-t border-border/50 transition-opacity ${vaultEnabled ? 'opacity-100' : 'opacity-40 pointer-events-none'}`}>
                            <label className="block text-sm font-semibold text-text-primary mb-2">Vault Root Path</label>
                            <div className="flex gap-2">
                                <input
                                    type="text"
                                    placeholder="C:/Users/User/Documents/MyVault"
                                    value={vaultPath}
                                    readOnly
                                    className="flex-1 bg-surface-elevated border border-border rounded-xl px-4 py-2.5 text-sm text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/10 transition-all placeholder:text-text-muted/50"
                                />
                                <button
                                    onClick={handleBrowseVault}
                                    className="px-5 py-2.5 bg-accent text-white font-medium rounded-xl hover:bg-accent-light transition-all shadow-sm active:scale-95 text-sm"
                                >
                                    Browse
                                </button>
                            </div>
                            {vaultPath && (
                                <p className="text-[11px] text-text-muted mt-2 ml-1">
                                    Logs to: <span className="font-mono">{vaultPath}/GameSense/2048/</span>
                                </p>
                            )}
                            {!vaultPath && (
                                <p className="text-[11px] text-text-muted mt-2 ml-1">Select your Obsidian vault root folder</p>
                            )}
                        </div>
                    </div>
                </section>

                {/* Appearance Section */}
                <section className="bg-surface-card border border-border rounded-2xl p-6 shadow-sm">
                    <h3 className="text-sm font-bold text-text-muted uppercase tracking-wider mb-5 flex items-center gap-2">
                        <Monitor size={16} className="text-accent" />
                        {t.appearance}
                    </h3>
                    <div className="flex items-center justify-between">
                        <div>
                            <p className="text-sm font-semibold text-text-primary">{t.appTheme}</p>
                            <p className="text-xs text-text-secondary mt-0.5">{t.selectTheme}</p>
                        </div>
                        <div className="flex bg-surface-elevated rounded-xl p-1 border border-border/50 relative isolate">
                            {themes.map((tItem) => (
                                <button
                                    key={tItem.value}
                                    onClick={() => handleThemeChange(tItem.value)}
                                    className={`relative px-4 py-2 rounded-lg transition-colors duration-200 flex items-center justify-center ${theme === tItem.value ? 'text-text-primary font-medium' : 'text-text-muted hover:text-text-primary'}`}
                                    title={tItem.label}
                                >
                                    <span className="relative z-10 flex items-center gap-2">
                                        {tItem.icon}
                                    </span>
                                    {theme === tItem.value && (
                                        <motion.div
                                            layoutId="activeTheme"
                                            className="absolute inset-0 bg-surface-card rounded-lg shadow-sm border border-border/10 z-0"
                                            transition={{ type: "spring", stiffness: 350, damping: 35 }}
                                        />
                                    )}
                                </button>
                            ))}
                        </div>
                    </div>
                </section>

                {/* General Settings */}
                <section className="bg-surface-card border border-border rounded-2xl p-6 shadow-sm">
                    <h3 className="text-sm font-bold text-text-muted uppercase tracking-wider mb-5 flex items-center gap-2">
                        <Globe size={16} className="text-accent" />
                        {t.general}
                    </h3>
                    <div className="space-y-6">
                        <div className="flex items-center justify-between">
                            <div>
                                <p className="text-sm font-semibold text-text-primary">{t.language}</p>
                                <p className="text-xs text-text-secondary mt-0.5">{t.changeLang}</p>
                            </div>
                            <div className="w-44">
                                <Select
                                    options={languages}
                                    value={language}
                                    onChange={handleLanguageChange}
                                />
                            </div>
                        </div>
                        <div className="flex items-center justify-between pt-6 border-t border-border/50">
                            <div>
                                <p className="text-sm font-semibold text-text-primary">{t.notifications}</p>
                                <p className="text-xs text-text-secondary mt-0.5">{t.showNotif}</p>
                            </div>
                            <div className="flex items-center">
                                <button
                                    onClick={handleNotificationToggle}
                                    className={`relative w-11 h-6 rounded-full transition-colors duration-200 focus:outline-none flex items-center ${notifications ? 'bg-accent border border-accent' : 'bg-surface-elevated border border-border'}`}
                                >
                                    <motion.div
                                        className="w-4 h-4 bg-white rounded-full shadow-sm ml-1"
                                        animate={{ x: notifications ? 18 : 0 }}
                                        transition={{ type: "spring", stiffness: 500, damping: 30 }}
                                    />
                                </button>
                            </div>
                        </div>
                    </div>
                </section>

                {/* Path Configuration */}
                <section className="bg-surface-card border border-border rounded-2xl p-6 shadow-sm">
                    <h3 className="text-sm font-bold text-text-muted uppercase tracking-wider mb-5 flex items-center gap-2">
                        <FolderOpen size={16} className="text-accent" />
                        Path Configuration
                    </h3>
                    <div className="space-y-6">
                        {/* ADB Path */}
                        <div>
                            <label className="block text-sm font-semibold text-text-primary mb-2">{t.customPath}</label>
                            <div className="flex gap-2">
                                <input
                                    type="text"
                                    placeholder={t.bundledAdb}
                                    value={adbPath}
                                    className="flex-1 bg-surface-elevated border border-border rounded-xl px-4 py-2.5 text-sm text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/10 transition-all placeholder:text-text-muted/50"
                                    readOnly
                                />
                                <button
                                    onClick={handleBrowseAdb}
                                    className="px-5 py-2.5 bg-surface-elevated border border-border rounded-xl text-text-secondary hover:text-text-primary hover:border-text-secondary transition-all hover:bg-surface-hover font-medium text-sm"
                                >
                                    {t.browse}
                                </button>
                            </div>
                            <p className="text-[11px] text-text-muted mt-2 ml-1">{t.leaveEmpty}</p>
                        </div>

                        {/* Capture Save Path */}
                        <div className="pt-5 border-t border-border/50">
                            <label className="block text-sm font-semibold text-text-primary mb-2">
                                Capture Save Path
                            </label>
                            <div className="flex gap-2">
                                <input
                                    type="text"
                                    placeholder="~/Pictures/GameSense Agent"
                                    value={captureSavePath}
                                    className="flex-1 bg-surface-elevated border border-border rounded-xl px-4 py-2.5 text-sm text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/10 transition-all placeholder:text-text-muted/50"
                                    readOnly
                                />
                                <button
                                    onClick={handleBrowseCapturePath}
                                    className="px-5 py-2.5 bg-accent text-white font-medium rounded-xl hover:bg-accent-light transition-all shadow-sm active:scale-95 text-sm"
                                >
                                    {t.browse}
                                </button>
                            </div>
                            <p className="text-[11px] text-text-muted mt-2 ml-1">{t.defaultSavePath}</p>
                        </div>

                        {/* Ask Before Save Toggle */}
                        <div className="pt-5 border-t border-border/50 flex items-center justify-between">
                            <div>
                                <p className="text-sm font-semibold text-text-primary">{t.askBeforeSave}</p>
                                <p className="text-xs text-text-secondary mt-0.5">{t.askBeforeSaveDesc}</p>
                            </div>
                            <div className="flex items-center">
                                <button
                                    onClick={handleAskBeforeSaveToggle}
                                    className={`relative w-11 h-6 rounded-full transition-colors duration-200 focus:outline-none flex items-center ${askBeforeSave ? 'bg-accent border border-accent' : 'bg-surface-elevated border border-border'}`}
                                >
                                    <motion.div
                                        className="w-4 h-4 bg-white rounded-full shadow-sm ml-1"
                                        animate={{ x: askBeforeSave ? 18 : 0 }}
                                        transition={{ type: "spring", stiffness: 500, damping: 30 }}
                                    />
                                </button>
                            </div>
                        </div>
                    </div>
                </section>

                {/* About Section - Simplified */}
                <section className="bg-surface-card border border-border rounded-xl p-6 shadow-sm overflow-hidden">
                    <div className="flex flex-col md:flex-row items-start gap-6">
                        <div className="flex-1">
                            <div className="flex items-center gap-3 mb-2">
                                <img src="/icon.png" alt="GameSense Agent" className="w-8 h-8 rounded-lg shadow-sm" />
                                <h2 className="text-xl font-bold text-text-primary">GameSense Agent</h2>
                                <span className="text-[10px] font-bold text-accent bg-accent/10 px-2 py-0.5 rounded border border-accent/20">
                                    v{t.version.split(':')[1].trim()}
                                </span>
                            </div>
                            <p className="text-sm text-text-secondary leading-relaxed mb-4">
                                {t.aboutDesc}
                            </p>
                            <div className="flex flex-wrap gap-2">
                                <button
                                    onClick={() => openUrl('https://github.com/h1dr0nn/adb-compass')}
                                    className="text-xs font-medium text-text-secondary hover:text-accent transition-colors bg-surface-elevated px-3 py-1.5 rounded-lg border border-border/50 flex items-center gap-2"
                                >
                                    <Github size={14} />
                                    GitHub
                                </button>
                                <button
                                    onClick={() => openUrl('https://github.com/h1dr0nn')}
                                    className="text-xs font-medium text-text-secondary hover:text-accent transition-colors bg-surface-elevated px-3 py-1.5 rounded-lg border border-border/50 flex items-center gap-2"
                                >
                                    <Globe size={14} />
                                    {t.officialWebsite}
                                </button>
                            </div>
                        </div>
                    </div>

                    <div className="mt-6 pt-6 border-t border-border/50 flex flex-wrap items-center justify-between gap-4">
                        <div className="flex items-center gap-2 text-xs text-text-secondary">
                            <span className="text-text-muted">{t.developedBy}</span>
                            <button
                                onClick={() => openUrl('https://github.com/h1dr0nn')}
                                className="font-bold hover:text-accent transition-colors"
                            >
                                h1dr0n
                            </button>
                        </div>
                        <div className="flex items-center gap-4">
                            <button
                                onClick={handleResetDefaults}
                                className="text-[11px] font-bold text-error/60 hover:text-error transition-colors flex items-center gap-1"
                            >
                                <RotateCcw size={12} />
                                {t.resetToDefaults}
                            </button>
                            <span className="text-text-muted/20">|</span>
                            <button
                                onClick={handleCheckUpdates}
                                className="text-[11px] font-bold text-accent/60 hover:text-accent transition-colors"
                            >
                                {t.checkUpdates}
                            </button>
                        </div>
                    </div>
                </section>

                <div className="flex justify-center pb-4">
                    <button
                        onClick={handleViewLogs}
                        className="flex items-center gap-2 text-[10px] font-bold text-text-muted hover:text-text-secondary transition-colors"
                    >
                        <FileText size={12} />
                        {t.viewLogs}
                    </button>
                </div>
            </div>
        </motion.div>
    );
}

// --- Provider-specific fields ---

interface ProviderFieldsProps {
    provider: AiProvider;
    apiKey: string;
    onApiKeyChange: (v: string) => void;
    onSaveApiKey: () => void;
    showApiKey: boolean;
    onToggleShow: () => void;
    model: string;
    onModelChange: (v: string) => void;
    ollamaUrl: string;
    onOllamaUrlChange: (v: string) => void;
    useCustomEndpoint: boolean;
    onToggleCustomEndpoint: () => void;
    customEndpoint: string;
    onCustomEndpointChange: (v: string) => void;
}

function ApiKeyField({ label, hint, apiKey, onChange, onSave, show, onToggleShow }: {
    label: string;
    hint: React.ReactNode;
    apiKey: string;
    onChange: (v: string) => void;
    onSave: () => void;
    show: boolean;
    onToggleShow: () => void;
}) {
    return (
        <div>
            <div className="flex items-center justify-between mb-2">
                <div>
                    <p className="text-sm font-semibold text-text-primary">{label}</p>
                    <p className="text-xs text-text-secondary mt-0.5">{hint}</p>
                </div>
                {apiKey ? (
                    <div className="flex items-center gap-1.5 text-xs text-success">
                        <CheckCircle2 size={14} /> Saved
                    </div>
                ) : (
                    <div className="flex items-center gap-1.5 text-xs text-text-muted">
                        <XCircle size={14} /> Not set
                    </div>
                )}
            </div>
            <div className="flex gap-2">
                <div className="relative flex-1">
                    <input
                        type={show ? 'text' : 'password'}
                        value={apiKey}
                        onChange={(e) => onChange(e.target.value)}
                        placeholder="sk-..."
                        className="w-full bg-surface-elevated border border-border rounded-xl px-4 py-2.5 pr-10 text-sm text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/10 transition-all placeholder:text-text-muted/50 font-mono"
                    />
                    <button
                        onClick={onToggleShow}
                        className="absolute right-3 top-1/2 -translate-y-1/2 text-text-muted hover:text-text-primary transition-colors"
                    >
                        {show ? <EyeOff size={15} /> : <Eye size={15} />}
                    </button>
                </div>
                <button
                    onClick={onSave}
                    disabled={!apiKey.trim()}
                    className="px-4 py-2.5 bg-accent text-white font-medium rounded-xl hover:bg-accent/90 transition-all shadow-sm active:scale-95 text-sm disabled:opacity-40 disabled:cursor-not-allowed"
                >
                    Save
                </button>
            </div>
        </div>
    );
}

function CustomEndpointToggle({ enabled, onToggle, value, onChange, placeholder }: {
    enabled: boolean;
    onToggle: () => void;
    value: string;
    onChange: (v: string) => void;
    placeholder: string;
}) {
    return (
        <div>
            <div className="flex items-center justify-between">
                <div>
                    <p className="text-sm font-semibold text-text-primary">Custom Endpoint</p>
                    <p className="text-xs text-text-secondary mt-0.5">Override the default API base URL</p>
                </div>
                <button
                    onClick={onToggle}
                    className={`relative w-11 h-6 rounded-full transition-colors duration-200 flex items-center ${enabled ? 'bg-accent border border-accent' : 'bg-surface-elevated border border-border'}`}
                >
                    <motion.div
                        className="w-4 h-4 bg-white rounded-full shadow-sm ml-1"
                        animate={{ x: enabled ? 18 : 0 }}
                        transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                    />
                </button>
            </div>
            <AnimatePresence>
                {enabled && (
                    <motion.div
                        initial={{ opacity: 0, height: 0 }}
                        animate={{ opacity: 1, height: 'auto' }}
                        exit={{ opacity: 0, height: 0 }}
                        transition={{ duration: 0.2 }}
                        className="overflow-hidden"
                    >
                        <input
                            type="text"
                            value={value}
                            onChange={(e) => onChange(e.target.value)}
                            placeholder={placeholder}
                            className="w-full mt-3 bg-surface-elevated border border-border rounded-xl px-4 py-2.5 text-sm text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/10 transition-all font-mono placeholder:text-text-muted/50"
                        />
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
}

function useModels(provider: AiProvider, apiKey: string, ollamaUrl: string, customEndpoint: string, useCustomEndpoint: boolean) {
    const [models, setModels] = useState<{ value: string; label: string }[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        if (provider === 'ollama') {
            const base = ollamaUrl.replace(/\/$/, '');
            setLoading(true);
            fetch(`${base}/api/tags`)
                .then((r) => r.json())
                .then((data) => {
                    const list = (data.models ?? []).map((m: { name: string }) => ({ value: m.name, label: m.name }));
                    setModels(list);
                })
                .catch(() => setModels([]))
                .finally(() => setLoading(false));
            return;
        }

        if (!apiKey.trim()) { setModels([]); return; }

        if (provider === 'gemini') {
            const base = (useCustomEndpoint && customEndpoint.trim())
                ? customEndpoint.replace(/\/$/, '')
                : 'https://generativelanguage.googleapis.com/v1beta';
            setLoading(true);
            fetch(`${base}/models?key=${apiKey}`)
                .then((r) => r.json())
                .then((data) => {
                    const list = (data.models ?? [])
                        .filter((m: { name: string; supportedGenerationMethods?: string[] }) =>
                            m.supportedGenerationMethods?.includes('generateContent'))
                        .map((m: { name: string; displayName?: string }) => ({
                            value: m.name.replace('models/', ''),
                            label: m.displayName ?? m.name.replace('models/', ''),
                        }));
                    setModels(list);
                })
                .catch(() => setModels([]))
                .finally(() => setLoading(false));
            return;
        }

        if (provider === 'openai') {
            const base = (useCustomEndpoint && customEndpoint.trim())
                ? customEndpoint.replace(/\/$/, '')
                : 'https://api.openai.com/v1';
            setLoading(true);
            fetch(`${base}/models`, { headers: { Authorization: `Bearer ${apiKey}` } })
                .then((r) => r.json())
                .then((data) => {
                    const list = (data.data ?? [])
                        .map((m: { id: string }) => ({ value: m.id, label: m.id }));
                    setModels(list);
                })
                .catch(() => setModels([]))
                .finally(() => setLoading(false));
        }
    }, [provider, apiKey, ollamaUrl, customEndpoint, useCustomEndpoint]);

    return { models, loading };
}

function ProviderFields({
    provider, apiKey, onApiKeyChange, onSaveApiKey,
    showApiKey, onToggleShow, model, onModelChange,
    ollamaUrl, onOllamaUrlChange,
    useCustomEndpoint, onToggleCustomEndpoint, customEndpoint, onCustomEndpointChange,
}: ProviderFieldsProps) {
    const { models, loading } = useModels(provider, apiKey, ollamaUrl, customEndpoint, useCustomEndpoint);

    const modelOptions = models;

    if (provider === 'gemini') {
        return (
            <>
                <CustomEndpointToggle
                    enabled={useCustomEndpoint}
                    onToggle={onToggleCustomEndpoint}
                    value={customEndpoint}
                    onChange={onCustomEndpointChange}
                    placeholder="https://generativelanguage.googleapis.com/v1beta"
                />
                <ApiKeyField
                    label="API Key"
                    hint={<>Get your key at <button onClick={() => openUrl('https://aistudio.google.com/apikey')} className="text-accent hover:underline">aistudio.google.com</button></>}
                    apiKey={apiKey}
                    onChange={onApiKeyChange}
                    onSave={onSaveApiKey}
                    show={showApiKey}
                    onToggleShow={onToggleShow}
                />
                <div className="flex items-center justify-between gap-4">
                    <div>
                        <p className="text-sm font-semibold text-text-primary">Model</p>
                        <p className="text-xs text-text-secondary mt-0.5">
                            {loading ? 'Fetching models...' : apiKey ? `${modelOptions.length} models available` : 'Enter API key to load models'}
                        </p>
                    </div>
                    <div className="w-52">
                        <Select options={modelOptions} value={model} onChange={onModelChange} />
                    </div>
                </div>
            </>
        );
    }

    if (provider === 'openai') {
        return (
            <>
                <CustomEndpointToggle
                    enabled={useCustomEndpoint}
                    onToggle={onToggleCustomEndpoint}
                    value={customEndpoint}
                    onChange={onCustomEndpointChange}
                    placeholder="https://api.openai.com/v1"
                />
                <ApiKeyField
                    label="API Key"
                    hint={<>Get your key at <button onClick={() => openUrl('https://platform.openai.com/api-keys')} className="text-accent hover:underline">platform.openai.com</button></>}
                    apiKey={apiKey}
                    onChange={onApiKeyChange}
                    onSave={onSaveApiKey}
                    show={showApiKey}
                    onToggleShow={onToggleShow}
                />
                <div className="flex items-center justify-between gap-4">
                    <div>
                        <p className="text-sm font-semibold text-text-primary">Model</p>
                        <p className="text-xs text-text-secondary mt-0.5">
                            {loading ? 'Fetching models...' : apiKey ? `${modelOptions.length} models available` : 'Enter API key to load models'}
                        </p>
                    </div>
                    <div className="w-52">
                        <Select options={modelOptions} value={model} onChange={onModelChange} />
                    </div>
                </div>
            </>
        );
    }

    // Ollama
    return (
        <>
            <div>
                <p className="text-sm font-semibold text-text-primary mb-2">Ollama Server URL</p>
                <input
                    type="text"
                    value={ollamaUrl}
                    onChange={(e) => onOllamaUrlChange(e.target.value)}
                    placeholder="http://localhost:11434"
                    className="w-full bg-surface-elevated border border-border rounded-xl px-4 py-2.5 text-sm text-text-primary focus:outline-none focus:border-accent focus:ring-2 focus:ring-accent/10 transition-all font-mono"
                />
                <p className="text-xs text-text-muted mt-1.5">Use a vision-capable model (e.g. llava, llava-llama3)</p>
            </div>
            <div className="flex items-center justify-between gap-4">
                <div>
                    <p className="text-sm font-semibold text-text-primary">Model</p>
                    <p className="text-xs text-text-secondary mt-0.5">
                        {loading ? 'Fetching models...' : modelOptions.length > 0 ? `${modelOptions.length} models found` : 'Could not reach Ollama'}
                    </p>
                </div>
                <div className="w-52">
                    {modelOptions.length > 0 ? (
                        <Select options={modelOptions} value={model} onChange={onModelChange} />
                    ) : (
                        <input
                            type="text"
                            value={model}
                            onChange={(e) => onModelChange(e.target.value)}
                            placeholder="llava"
                            className="w-full bg-surface-elevated border border-border rounded-lg px-3 py-2 text-sm text-text-primary focus:outline-none focus:border-accent transition-all font-mono"
                        />
                    )}
                </div>
            </div>
        </>
    );
}
