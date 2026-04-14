import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Smartphone, Zap, Hand, Gauge } from 'lucide-react';
import { ScreenCapture } from '../device/ScreenCapture';
import { AgentControls } from './AgentControls';
import { ThinkingDisplay } from './ThinkingDisplay';
import { MoveHistory } from './MoveHistory';
import { Select } from '../ui/Select';
import type { DeviceInfo, AgentConfig, AgentStateSnapshot } from '../../types';

/** Parse a tap action string like "tap:45%:60%" into {x, y} percentages (0–1). */
function parseTapAction(action: string): { x: number; y: number } | null {
  const m = action.match(/^tap:([\d.]+)%:([\d.]+)%$/);
  if (!m) return null;
  return { x: parseFloat(m[1]) / 100, y: parseFloat(m[2]) / 100 };
}

interface AgentTabProps {
  device: DeviceInfo | null;
  devices: DeviceInfo[];
  onDeviceChange: (device: DeviceInfo) => void;
}

const INITIAL_STATE: AgentStateSnapshot = {
  status: 'Idle',
  step: 0,
  history: [],
  last_reasoning: '',
  game_state: null,
  error_message: null,
  last_action: null,
};

function MiniToggle({ active, onClick, icon, label }: { active: boolean; onClick: () => void; icon: React.ReactNode; label: string }) {
  return (
    <button
      onClick={onClick}
      title={label}
      className={`flex items-center gap-1.5 px-2.5 py-2 rounded-lg text-xs font-medium transition-all ${
        active
          ? 'bg-accent/10 text-accent border border-accent/30'
          : 'bg-surface-elevated text-text-muted border border-border hover:text-text-primary'
      }`}
    >
      {icon}
      {label}
    </button>
  );
}

export function AgentTab({ device, devices, onDeviceChange }: AgentTabProps) {
  const [agentState, setAgentState] = useState<AgentStateSnapshot>(INITIAL_STATE);
  const [maxSteps, setMaxSteps] = useState(100);
  const [livePreview, setLivePreview] = useState(false);
  const [enableTouch, setEnableTouch] = useState(false);
  const [showFps, setShowFps] = useState(true);
  const [agentCursor, setAgentCursor] = useState<{ x: number; y: number; key: number } | null>(null);

  useEffect(() => {
    const unlisten = listen<AgentStateSnapshot>('agent-state-changed', (event) => {
      const state = event.payload;
      setAgentState(state);
        // Show cursor as soon as agent starts acting (last_action set before execute)
      if (state.status === 'Acting' && state.last_action) {
        const pos = parseTapAction(state.last_action);
        if (pos) {
          setAgentCursor({ ...pos, key: Date.now() });
        }
      }
    });
    invoke<AgentStateSnapshot>('get_agent_state')
      .then(setAgentState)
      .catch(() => { });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const handleStart = useCallback((config: AgentConfig) => {
    setMaxSteps(config.max_steps);
    invoke('start_agent', { config }).catch((err) => {
      setAgentState((prev) => ({ ...prev, status: 'Error', error_message: String(err) }));
    });
  }, []);

  const handleStop = useCallback(() => {
    invoke('stop_agent').catch(() => { });
  }, []);

  const connectedDevices = devices.filter(d => d.status === 'Device');
  const deviceOptions = connectedDevices.map(d => ({
    value: d.id,
    label: d.model || d.id,
    icon: <Smartphone size={14} />,
  }));

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-text-primary">Agent</h2>
        <div className="flex items-center gap-3">
          {device && (
            <div className="flex items-center gap-1.5">
              <MiniToggle active={livePreview} onClick={() => setLivePreview(!livePreview)} icon={<Zap size={12} />} label="Live" />
              <MiniToggle active={enableTouch} onClick={() => setEnableTouch(!enableTouch)} icon={<Hand size={12} />} label="Touch" />
              <MiniToggle active={showFps} onClick={() => setShowFps(!showFps)} icon={<Gauge size={12} />} label="FPS" />
            </div>
          )}
          <Select
          options={deviceOptions}
          value={device?.id ?? ''}
          onChange={(id) => {
            const selected = connectedDevices.find(d => d.id === id);
            if (selected) onDeviceChange(selected);
          }}
          placeholder={connectedDevices.length === 0 ? 'No devices connected' : 'Select device...'}
          className="w-52"
        />
        </div>
      </div>

      {/* Content */}
      {!device ? (
        <div className="flex-1 flex flex-col items-center justify-center text-text-muted gap-3">
          <Smartphone size={48} className="opacity-30" />
          <p className="text-sm">Select a device to start the agent</p>
        </div>
      ) : (
        <div className="flex-1 flex gap-4 min-h-0">
          <div className="shrink-0 h-full">
            <ScreenCapture
              device={device}
              compact
              externalLive={livePreview}
              externalTouch={enableTouch}
              externalShowFps={showFps}
              onLiveChange={setLivePreview}
              agentCursor={agentCursor}
            />
          </div>
          <div className="flex-1 min-w-0 flex flex-col gap-3 overflow-y-auto custom-scrollbar">
          <AgentControls
            deviceId={device.id}
            screenWidth={Number(localStorage.getItem(`screen_w_${device.id}`) || 1080)}
            screenHeight={Number(localStorage.getItem(`screen_h_${device.id}`) || 1920)}
            status={agentState.status}
            step={agentState.step}
            maxSteps={maxSteps}
            errorMessage={agentState.error_message}
            onStart={handleStart}
            onStop={handleStop}
          />
          <ThinkingDisplay
            status={agentState.status}
            reasoning={agentState.last_reasoning}
            gameState={agentState.game_state}
          />
          <MoveHistory history={agentState.history} />
          </div>
        </div>
      )}
    </div>
  );
}
