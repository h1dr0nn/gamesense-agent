import { useState, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { MirrorWindow } from './components/modals/MirrorWindow';
import { motion, AnimatePresence } from 'framer-motion';
import { Toaster } from 'sonner';
import { useDevices } from './hooks/useDevices';
import { NavRail } from './components/NavRail';
import type { NavView } from './components/NavRail';
import { DeviceList } from './components/DeviceList';
import { Settings } from './components/Settings';
import { LogcatView } from './components/LogcatView';
import { DeviceDetailView } from './components/DeviceDetailView';
import { AgentTab } from './components/agent/AgentTab';
import { ManualConnectModal } from './components/modals/ManualConnectModal';
import { useTheme } from './contexts/ThemeContext';
import { TitleBar } from './components/TitleBar';
import { DeviceProvider } from './contexts/DeviceContext';
import { DeviceCacheProvider } from './contexts/DeviceCacheContext';
import type { DeviceInfo } from './types';

function AppContent() {
  const { devices, loading, error, refreshDevices, removeDevice } = useDevices();
  const [activeView, setActiveView] = useState<NavView>('devices');
  const [selectedDevice, setSelectedDevice] = useState<DeviceInfo | null>(null);
  const [showManualConnect, setShowManualConnect] = useState(false);
  const [agentDevice, setAgentDevice] = useState<DeviceInfo | null>(null);
  const { resolvedTheme } = useTheme();

  const handleNavigate = (view: NavView) => {
    setActiveView(view);
    setSelectedDevice(null);
  };

  const handleDeviceSelect = (device: DeviceInfo) => {
    setSelectedDevice(device);
    setActiveView('devices');
  };

  const getViewKey = () => {
    if (selectedDevice) return `device-${selectedDevice.id}`;
    return activeView;
  };

  return (
    <div className="flex flex-col h-screen bg-surface-bg text-text-primary font-sans overflow-hidden">
      <Toaster
        position="bottom-right"
        theme={resolvedTheme === 'dark' ? 'dark' : 'light'}
        toastOptions={{
          className: 'bg-surface-card border border-border text-text-primary shadow-xl',
        }}
      />

      <TitleBar />

      <div className="flex-1 flex min-h-0">
        <NavRail activeView={activeView} onNavigate={handleNavigate} />

        <div className="flex-1 min-w-0 overflow-hidden relative">
          {/* AgentTab is always mounted to keep the listener + agent state alive across tab switches */}
          <div className={`absolute inset-0 px-8 py-6 overflow-hidden ${activeView === 'agent' && !selectedDevice ? '' : 'hidden'}`}>
            <AgentTab
              device={agentDevice}
              devices={devices}
              onDeviceChange={setAgentDevice}
            />
          </div>

          <AnimatePresence mode="wait">
            {(activeView !== 'agent' || selectedDevice) && (
              <motion.div
                key={getViewKey()}
                className="absolute inset-0 px-8 py-6 overflow-hidden"
                initial={{ opacity: 0, scale: 0.98 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.98 }}
                transition={{ duration: 0.2 }}
              >
                {selectedDevice ? (
                  <DeviceDetailView device={selectedDevice} onBack={() => setSelectedDevice(null)} />
                ) : activeView === 'settings' ? (
                  <Settings onBack={() => setActiveView('devices')} />
                ) : activeView === 'logcat' ? (
                  <LogcatView />
                ) : (
                  <div className="h-full flex flex-col">
                    <div className="flex-1 overflow-y-auto custom-scrollbar pr-2">
                      <DeviceList
                        devices={devices}
                        loading={loading}
                        error={error}
                        onRefresh={refreshDevices}
                        onDeviceSelect={handleDeviceSelect}
                        onRemove={removeDevice}
                        onAddDevice={() => setShowManualConnect(true)}
                      />
                    </div>
                    <div className="mt-4 text-center text-text-muted text-sm">
                      <p>Connect an Android device to get started</p>
                    </div>
                  </div>
                )}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>

      {showManualConnect && <ManualConnectModal onClose={() => setShowManualConnect(false)} />}
    </div>
  );
}

function App() {
  const [windowLabel, setWindowLabel] = useState<string>('');

  useEffect(() => {
    setWindowLabel(getCurrentWindow().label);
  }, []);

  if (windowLabel.startsWith('mirror-')) {
    return (
      <div className="h-screen bg-black overflow-hidden font-sans">
        <MirrorWindow />
      </div>
    );
  }

  return (
    <DeviceProvider>
      <DeviceCacheProvider>
        <AppContent />
      </DeviceCacheProvider>
    </DeviceProvider>
  );
}

export default App;
