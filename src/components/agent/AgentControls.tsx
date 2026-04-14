import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Play, Square, Settings, RefreshCw } from 'lucide-react';
import type { AgentStatus, AgentConfig } from '../../types';

interface AgentControlsProps {
  deviceId: string;
  screenWidth: number;
  screenHeight: number;
  status: AgentStatus;
  step: number;
  maxSteps: number;
  errorMessage: string | null;
  onStart: (config: AgentConfig) => void;
  onStop: () => void;
}

const STATUS_COLORS: Record<AgentStatus, string> = {
  Idle: 'bg-text-muted',
  Observing: 'bg-blue-500 animate-pulse',
  Thinking: 'bg-amber-500 animate-pulse',
  Acting: 'bg-green-500',
  Waiting: 'bg-text-muted animate-pulse',
  Won: 'bg-green-500',
  GameOver: 'bg-red-500',
  Error: 'bg-red-500',
  Stopped: 'bg-text-muted',
};

const STATUS_LABELS: Record<AgentStatus, string> = {
  Idle: 'Ready',
  Observing: 'Capturing screen...',
  Thinking: 'Analyzing...',
  Acting: 'Executing action...',
  Waiting: 'Waiting for animation...',
  Won: 'Game Won!',
  GameOver: 'Game Over',
  Error: 'Error',
  Stopped: 'Stopped',
};

function readAiSettings() {
  const provider = localStorage.getItem('ai_provider') ?? 'gemini';
  const apiKey = localStorage.getItem('gemini_api_key') ?? '';
  const model = localStorage.getItem('gemini_model') ?? 'gemini-2.0-flash';
  const useCustomEndpoint = localStorage.getItem('use_custom_endpoint') === 'true';
  const customEndpoint = localStorage.getItem('custom_endpoint') ?? '';
  const ollamaUrl = localStorage.getItem('ollama_url') ?? 'http://localhost:11434';
  const vaultEnabled = localStorage.getItem('vault_enabled') === 'true';
  const vaultPath = localStorage.getItem('vault_path') ?? '';

  let baseUrl: string | undefined;
  if (provider === 'openai' && useCustomEndpoint && customEndpoint) {
    baseUrl = customEndpoint;
  } else if (provider === 'openai') {
    baseUrl = 'https://api.openai.com/v1';
  } else if (provider === 'ollama') {
    baseUrl = `${ollamaUrl}/v1`;
  }

  const vaultPathResolved = vaultEnabled && vaultPath ? vaultPath : undefined;

  return { provider, apiKey, model, baseUrl, vaultPath: vaultPathResolved };
}

export function AgentControls({
  deviceId,
  screenWidth,
  screenHeight,
  status,
  step,
  maxSteps,
  errorMessage,
  onStart,
  onStop,
}: AgentControlsProps) {
  const [maxStepsInput, setMaxStepsInput] = useState(100);
  // gameId = package name (vault folder), gameName = human-readable (prompt)
  const [gameId, setGameId] = useState(() => localStorage.getItem('agent_game_id') ?? '');
  const [gameName, setGameName] = useState(() => localStorage.getItem('agent_game_name') ?? '');
  const [detectingGame, setDetectingGame] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [aiSettings, setAiSettings] = useState(readAiSettings);

  // Auto-detect foreground app when component mounts or deviceId changes
  useEffect(() => {
    detectForegroundGame();
  }, [deviceId]);

  const detectForegroundGame = () => {
    if (!deviceId) return;
    setDetectingGame(true);
    invoke<string>('get_foreground_app', { deviceId })
      .then((pkg) => {
        if (!pkg?.trim()) return;
        const pkgTrimmed = pkg.trim();
        setGameId(pkgTrimmed);
        localStorage.setItem('agent_game_id', pkgTrimmed);
        // Fetch human-readable label
        return invoke<string>('get_app_label', { deviceId, packageName: pkgTrimmed })
          .then((label) => {
            const name = label?.trim() || pkgTrimmed;
            setGameName(name);
            localStorage.setItem('agent_game_name', name);
          })
          .catch(() => {
            setGameName(pkgTrimmed);
            localStorage.setItem('agent_game_name', pkgTrimmed);
          });
      })
      .catch(() => {})
      .finally(() => setDetectingGame(false));
  };

  const isRunning = !['Idle', 'Won', 'GameOver', 'Error', 'Stopped'].includes(status);

  // Refresh settings when panel opens (in case user changed them)
  useEffect(() => {
    if (showSettings) {
      setAiSettings(readAiSettings());
    }
  }, [showSettings]);

  const hasApiKey = aiSettings.provider === 'ollama' || !!aiSettings.apiKey.trim();
  const canStart = hasApiKey && !!gameName.trim();

  const handleStart = () => {
    if (!canStart) return;
    const settings = readAiSettings();
    onStart({
      device_id: deviceId,
      api_key: settings.apiKey,
      model: settings.model,
      max_steps: maxStepsInput,
      delay_between_moves: 500,
      screen_width: screenWidth,
      screen_height: screenHeight,
      base_url: settings.baseUrl,
      vault_path: settings.vaultPath,
      game_name: gameName.trim() || gameId.trim(),
      game_id: gameId.trim() || gameName.trim(),
    });
  };

  const providerLabel = {
    gemini: 'Gemini',
    openai: aiSettings.baseUrl && aiSettings.baseUrl !== 'https://api.openai.com/v1'
      ? 'OpenAI (Custom)'
      : 'OpenAI',
    ollama: 'Ollama',
  }[aiSettings.provider] ?? aiSettings.provider;

  return (
    <div className="bg-surface-card border border-border rounded-xl p-4 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-semibold text-text-primary">Agent Control</h3>
        <button
          onClick={() => setShowSettings(!showSettings)}
          className="p-1.5 rounded-lg hover:bg-surface-elevated text-text-muted hover:text-text-primary transition-colors"
        >
          <Settings size={14} />
        </button>
      </div>

      {/* Settings summary / expand */}
      {showSettings ? (
        <div className="space-y-2 pb-3 border-b border-border text-xs text-text-secondary">
          <div className="flex justify-between">
            <span className="text-text-muted">Provider</span>
            <span className="font-medium text-text-primary">{providerLabel}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-text-muted">Model</span>
            <span className="font-medium text-text-primary truncate max-w-[140px]">{aiSettings.model || '—'}</span>
          </div>
          {aiSettings.baseUrl && (
            <div className="flex justify-between">
              <span className="text-text-muted">Endpoint</span>
              <span className="font-medium text-text-primary truncate max-w-[140px]">{aiSettings.baseUrl}</span>
            </div>
          )}
          <div className="flex justify-between">
            <span className="text-text-muted">API Key</span>
            <span className={hasApiKey ? 'text-green-400' : 'text-red-400'}>
              {aiSettings.provider === 'ollama' ? 'Not required' : hasApiKey ? 'Set' : 'Not set'}
            </span>
          </div>
          <p className="text-text-muted pt-1">
            Change AI settings in <span className="text-accent">Settings → AI Provider</span>
          </p>
        </div>
      ) : (
        <div className="flex items-center justify-between text-xs pb-1">
          <span className="text-text-muted">{providerLabel}</span>
          <span className="text-text-muted truncate max-w-[150px]">{aiSettings.model || '—'}</span>
        </div>
      )}

      {/* Game */}
      <div>
        <label className="text-xs text-text-muted block mb-1">Game (foreground app)</label>
        <div className="flex gap-1.5">
          <div className="flex-1 min-w-0 bg-surface-elevated border border-border rounded-lg px-3 py-1.5 text-sm">
            {gameName || gameId ? (
              <>
                <div className="text-text-primary truncate">{gameName || gameId}</div>
                {gameName && gameId && gameName !== gameId && (
                  <div className="text-text-muted text-xs font-mono truncate mt-0.5">{gameId}</div>
                )}
              </>
            ) : (
              <span className="text-text-muted">{detectingGame ? 'Detecting…' : 'No app detected'}</span>
            )}
          </div>
          <button
            onClick={detectForegroundGame}
            disabled={isRunning || detectingGame}
            title="Detect foreground app"
            className="p-1.5 rounded-lg bg-surface-elevated border border-border hover:border-accent text-text-muted hover:text-accent transition-colors disabled:opacity-40 shrink-0"
          >
            <RefreshCw size={14} className={detectingGame ? 'animate-spin' : ''} />
          </button>
        </div>
      </div>

      {/* Max Steps */}
      <div>
        <label className="text-xs text-text-muted block mb-1">Max Steps</label>
        <input
          type="number"
          value={maxStepsInput}
          onChange={(e) => setMaxStepsInput(Number(e.target.value))}
          min={1}
          max={500}
          className="w-full bg-surface-elevated border border-border rounded-lg px-3 py-1.5 text-sm focus:outline-none focus:border-accent"
        />
      </div>

      {/* Start/Stop buttons */}
      <div className="flex gap-2">
        {isRunning ? (
          <button
            onClick={onStop}
            className="flex-1 flex items-center justify-center gap-2 py-2.5 bg-red-500/10 hover:bg-red-500/20 border border-red-500/30 text-red-500 rounded-lg transition-all text-sm font-medium"
          >
            <Square size={14} />
            Stop
          </button>
        ) : (
          <button
            onClick={handleStart}
            disabled={!canStart}
            className="flex-1 flex items-center justify-center gap-2 py-2.5 bg-accent/10 hover:bg-accent/20 border border-accent/30 text-accent rounded-lg transition-all text-sm font-medium disabled:opacity-40"
          >
            <Play size={14} />
            Start Agent
          </button>
        )}
      </div>

      {/* Status */}
      <div className="flex items-center justify-between text-xs">
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${STATUS_COLORS[status]}`} />
          <span className="text-text-secondary">{STATUS_LABELS[status]}</span>
        </div>
        {isRunning && (
          <span className="text-text-muted font-mono">
            {step} / {maxSteps}
          </span>
        )}
      </div>

      {/* Error message */}
      {errorMessage && status === 'Error' && (
        <div className="text-xs text-red-400 bg-red-500/10 border border-red-500/20 rounded-lg p-2">
          {errorMessage}
        </div>
      )}
    </div>
  );
}
