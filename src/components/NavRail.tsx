import { motion } from 'framer-motion';
import { Smartphone, Bot, FileText, Settings } from 'lucide-react';

export type NavView = 'devices' | 'agent' | 'logcat' | 'settings';

interface NavRailProps {
  activeView: NavView;
  onNavigate: (view: NavView) => void;
}

interface NavItem {
  id: NavView;
  icon: React.ReactNode;
  label: string;
}

const TOP_NAV: NavItem[] = [
  { id: 'devices', icon: <Smartphone size={20} />, label: 'Devices' },
  { id: 'agent', icon: <Bot size={20} />, label: 'Agent' },
  { id: 'logcat', icon: <FileText size={20} />, label: 'Logcat' },
];

export function NavRail({ activeView, onNavigate }: NavRailProps) {
  return (
    <nav className="w-16 border-r border-border bg-surface-bg flex flex-col items-center py-4 gap-1 z-20 shrink-0">
      <div className="flex-1 flex flex-col items-center gap-1">
        {TOP_NAV.map((item) => (
          <NavButton
            key={item.id}
            item={item}
            isActive={activeView === item.id}
            onClick={() => onNavigate(item.id)}
          />
        ))}
      </div>

      <NavButton
        item={{ id: 'settings', icon: <Settings size={20} />, label: 'Settings' }}
        isActive={activeView === 'settings'}
        onClick={() => onNavigate('settings')}
      />
    </nav>
  );
}

interface NavButtonProps {
  item: NavItem;
  isActive: boolean;
  onClick: () => void;
}

function NavButton({ item, isActive, onClick }: NavButtonProps) {
  return (
    <button
      onClick={onClick}
      title={item.label}
      className={`relative w-10 h-10 flex items-center justify-center rounded-xl transition-all duration-200 group ${
        isActive
          ? 'text-accent bg-accent/10'
          : 'text-text-muted hover:text-text-primary hover:bg-surface-elevated'
      }`}
    >
      {isActive && (
        <motion.div
          layoutId="navRailActive"
          className="absolute inset-0 bg-accent/10 rounded-xl border border-accent/20"
          transition={{ type: 'spring', stiffness: 400, damping: 30 }}
        />
      )}
      <span className="relative z-10">{item.icon}</span>

      <span className="absolute left-full ml-2 px-2 py-1 bg-surface-card border border-border rounded-md text-xs text-text-primary whitespace-nowrap opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity z-50 shadow-lg">
        {item.label}
      </span>
    </button>
  );
}
