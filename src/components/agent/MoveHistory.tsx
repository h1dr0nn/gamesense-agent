import { useRef, useEffect } from 'react';
import { History } from 'lucide-react';
import type { AgentMove } from '../../types';

interface MoveHistoryProps {
  history: AgentMove[];
}

const ACTION_ICONS: Record<string, string> = {
  swipe_up: '\u2191',
  swipe_down: '\u2193',
  swipe_left: '\u2190',
  swipe_right: '\u2192',
};

export function MoveHistory({ history }: MoveHistoryProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [history.length]);

  return (
    <div className="bg-surface-card border border-border rounded-xl p-4 flex flex-col min-h-0">
      <div className="flex items-center gap-2 mb-3">
        <History size={14} className="text-text-muted" />
        <h3 className="text-sm font-semibold text-text-primary">
          Move History
          {history.length > 0 && (
            <span className="text-text-muted font-normal ml-1">({history.length})</span>
          )}
        </h3>
      </div>

      <div className="flex-1 overflow-y-auto custom-scrollbar space-y-1 max-h-60">
        {history.length === 0 ? (
          <div className="text-xs text-text-muted italic py-2">No moves yet</div>
        ) : (
          [...history].reverse().map((move) => (
            <div
              key={move.step}
              className="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-surface-elevated transition-colors text-xs"
            >
              <span className="text-text-muted font-mono w-6 text-right">#{move.step}</span>
              <span className="text-lg leading-none w-5 text-center">
                {ACTION_ICONS[move.action] || '?'}
              </span>
              <span className="text-text-secondary flex-1 truncate">{move.action}</span>
              <span className="text-text-muted font-mono">
                {(move.confidence * 100).toFixed(0)}%
              </span>
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
}
