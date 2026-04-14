import { Brain } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import type { AgentStatus, GameState } from '../../types';

interface ThinkingDisplayProps {
  status: AgentStatus;
  reasoning: string;
  gameState: GameState | null;
}

export function ThinkingDisplay({ status, reasoning, gameState }: ThinkingDisplayProps) {
  const isThinking = status === 'Thinking';

  return (
    <div className="bg-surface-card border border-border rounded-xl p-4 space-y-3">
      <div className="flex items-center gap-2">
        <Brain size={14} className={isThinking ? 'text-amber-500 animate-pulse' : 'text-text-muted'} />
        <h3 className="text-sm font-semibold text-text-primary">Thinking</h3>
      </div>

      <AnimatePresence mode="wait">
        {reasoning ? (
          <motion.div
            key={reasoning}
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className="text-xs text-text-secondary leading-relaxed bg-surface-elevated rounded-lg p-3 max-h-32 overflow-y-auto custom-scrollbar"
          >
            {reasoning}
          </motion.div>
        ) : (
          <div className="text-xs text-text-muted italic">
            {status === 'Idle' ? 'Start the agent to begin' : 'Waiting for analysis...'}
          </div>
        )}
      </AnimatePresence>

      {/* Game state summary */}
      {gameState && (
        <div className="flex items-center gap-3 text-xs text-text-muted">
          <span>Score: <span className="text-text-primary font-mono">{gameState.score}</span></span>
          <span>Status: <span className={`font-medium ${gameState.status === 'playing' ? 'text-green-400' : gameState.status === 'won' ? 'text-amber-400' : 'text-red-400'}`}>
            {gameState.status}
          </span></span>
        </div>
      )}
    </div>
  );
}
