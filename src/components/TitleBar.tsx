import { useState, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Square, X } from 'lucide-react';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const win = getCurrentWindow();

  useEffect(() => {
    win.isMaximized().then(setIsMaximized);
    const unlisten = win.onResized(() => {
      win.isMaximized().then(setIsMaximized);
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const handleMinimize = () => win.minimize();
  const handleToggleMax = () => win.toggleMaximize();
  const handleClose = () => win.close();

  return (
    <div
      data-tauri-drag-region
      className="h-11 flex items-center justify-between px-4 bg-surface-bg border-b border-border select-none shrink-0 z-[9999] relative"
    >
      {/* App identity */}
      <div className="flex items-center gap-2.5 pointer-events-none">
        <img src="/icon.png" alt="" className="w-5 h-5 rounded-md" />
        <span className="text-sm font-semibold text-text-primary">GameSense Agent</span>
        <span className="text-xs text-text-muted font-mono">v0.0.1</span>
      </div>

      {/* Window controls */}
      <div className="flex items-center gap-1">
        <WinButton onClick={handleMinimize} label="Minimize">
          <Minus size={13} />
        </WinButton>
        <WinButton onClick={handleToggleMax} label={isMaximized ? 'Restore' : 'Maximize'}>
          <Square size={11} strokeWidth={2.5} />
        </WinButton>
        <WinButton onClick={handleClose} label="Close" danger>
          <X size={13} />
        </WinButton>
      </div>
    </div>
  );
}

interface WinButtonProps {
  onClick: () => void;
  label: string;
  danger?: boolean;
  children: React.ReactNode;
}

function WinButton({ onClick, label, danger, children }: WinButtonProps) {
  return (
    <button
      onClick={onClick}
      title={label}
      className={`w-8 h-7 flex items-center justify-center rounded-md transition-colors ${
        danger
          ? 'text-text-muted hover:bg-red-500 hover:text-white'
          : 'text-text-muted hover:bg-surface-elevated hover:text-text-primary'
      }`}
    >
      {children}
    </button>
  );
}
