import { useState, useRef, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronDown, Check } from 'lucide-react';

interface SelectOption {
    value: string;
    label: string;
    icon?: React.ReactNode;
}

interface SelectProps {
    options: SelectOption[];
    value: string;
    onChange: (value: string) => void;
    placeholder?: string;
    className?: string;
    /** Enable combobox mode (type-to-search in the trigger). Defaults to true when options > 10. */
    searchable?: boolean;
}

export function Select({ options, value, onChange, placeholder = "Select...", className = "", searchable }: SelectProps) {
    const isCombobox = searchable ?? options.length > 10;

    const selectedOption = options.find(opt => opt.value === value);
    const [isOpen, setIsOpen] = useState(false);
    // inputText mirrors what the user is typing; resets to selected label on close
    const [inputText, setInputText] = useState(selectedOption?.label ?? '');
    const containerRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const listRef = useRef<HTMLDivElement>(null);

    // Keep inputText in sync when value or options change (e.g. options load after mount)
    useEffect(() => {
        if (!isOpen) {
            setInputText(selectedOption?.label ?? value);
        }
    }, [value, selectedOption?.label, isOpen]);

    const filtered = isCombobox && inputText && inputText !== selectedOption?.label
        ? options.filter(o =>
            o.label.toLowerCase().includes(inputText.toLowerCase()) ||
            o.value.toLowerCase().includes(inputText.toLowerCase())
          )
        : options;

    // Close on outside click
    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
                closeAndReset();
            }
        };
        document.addEventListener('mousedown', handleClickOutside);
        return () => document.removeEventListener('mousedown', handleClickOutside);
    }, [selectedOption]);

    // Scroll selected item into view when list opens
    useEffect(() => {
        if (!isOpen || !listRef.current) return;
        const el = listRef.current.querySelector('[data-selected="true"]');
        if (el) el.scrollIntoView({ block: 'nearest' });
    }, [isOpen]);

    const closeAndReset = () => {
        setIsOpen(false);
        setInputText(selectedOption?.label ?? '');
    };

    const handleSelect = (optValue: string) => {
        onChange(optValue);
        setIsOpen(false);
        const opt = options.find(o => o.value === optValue);
        setInputText(opt?.label ?? '');
    };

    // --- Combobox trigger (input) ---
    if (isCombobox) {
        return (
            <div className={`relative ${className}`} ref={containerRef}>
                <div className={`flex items-center bg-surface-elevated border rounded-lg transition-colors ${isOpen ? 'border-accent ring-1 ring-accent/20' : 'border-border hover:border-accent/50'}`}>
                    <input
                        ref={inputRef}
                        type="text"
                        value={inputText}
                        placeholder={placeholder}
                        onChange={(e) => {
                            setInputText(e.target.value);
                            setIsOpen(true);
                        }}
                        onFocus={() => {
                            setInputText('');   // clear so the full list shows immediately
                            setIsOpen(true);
                        }}
                        onKeyDown={(e) => {
                            if (e.key === 'Escape') { closeAndReset(); inputRef.current?.blur(); }
                            if (e.key === 'Enter' && filtered.length === 1) handleSelect(filtered[0].value);
                            if (e.key === 'ArrowDown') { e.preventDefault(); setIsOpen(true); }
                        }}
                        className="flex-1 min-w-0 px-3 py-2 bg-transparent text-sm text-text-primary placeholder:text-text-muted outline-none"
                    />
                    <button
                        tabIndex={-1}
                        onClick={() => { setIsOpen(!isOpen); inputRef.current?.focus(); }}
                        className="px-2 text-text-secondary"
                    >
                        <ChevronDown size={16} className={`transition-transform duration-200 ${isOpen ? 'rotate-180' : ''}`} />
                    </button>
                </div>

                <AnimatePresence>
                    {isOpen && (
                        <motion.div
                            initial={{ opacity: 0, y: -8, scale: 0.97 }}
                            animate={{ opacity: 1, y: 0, scale: 1 }}
                            exit={{ opacity: 0, y: -8, scale: 0.97 }}
                            transition={{ duration: 0.12, ease: "easeOut" }}
                            className="absolute z-50 w-full mt-1 bg-surface-elevated border border-border rounded-lg shadow-xl overflow-hidden"
                        >
                            {/* result count */}
                            {inputText && inputText !== selectedOption?.label && (
                                <div className="px-3 py-1.5 border-b border-border flex items-center justify-between">
                                    <span className="text-xs text-text-muted">{filtered.length} result{filtered.length !== 1 ? 's' : ''}</span>
                                </div>
                            )}
                            <div ref={listRef} className="overflow-y-auto custom-scrollbar py-1 max-h-56">
                                {filtered.length === 0 ? (
                                    <div className="px-3 py-4 text-sm text-text-muted text-center">
                                        No match for "{inputText}"
                                    </div>
                                ) : (
                                    filtered.map((option) => (
                                        <button
                                            key={option.value}
                                            data-selected={option.value === value}
                                            onMouseDown={(e) => { e.preventDefault(); handleSelect(option.value); }}
                                            className={`w-full flex items-center justify-between px-3 py-2 text-sm text-left hover:bg-surface-hover transition-colors
                                                ${option.value === value ? 'text-accent bg-accent/5 font-medium' : 'text-text-primary'}`}
                                        >
                                            <span className="flex items-center gap-2 truncate">
                                                {option.icon}
                                                <span className="truncate">{option.label}</span>
                                            </span>
                                            {option.value === value && <Check size={14} className="shrink-0 ml-2" />}
                                        </button>
                                    ))
                                )}
                            </div>
                        </motion.div>
                    )}
                </AnimatePresence>
            </div>
        );
    }

    // --- Regular select (few options, no search) ---
    return (
        <div className={`relative ${className}`} ref={containerRef}>
            <motion.button
                whileTap={{ scale: 0.98 }}
                onClick={() => setIsOpen(!isOpen)}
                className={`w-full flex items-center justify-between px-3 py-2 bg-surface-elevated border border-border rounded-lg text-sm text-text-primary hover:border-accent/50 transition-colors ${isOpen ? 'border-accent ring-1 ring-accent/20' : ''}`}
            >
                <span className="flex items-center gap-2 truncate">
                    {selectedOption?.icon}
                    {selectedOption
                        ? selectedOption.label
                        : <span className="text-text-muted">{placeholder}</span>}
                </span>
                <ChevronDown
                    size={16}
                    className={`text-text-secondary transition-transform duration-200 shrink-0 ml-2 ${isOpen ? 'rotate-180' : ''}`}
                />
            </motion.button>

            <AnimatePresence>
                {isOpen && (
                    <motion.div
                        initial={{ opacity: 0, y: -8, scale: 0.97 }}
                        animate={{ opacity: 1, y: 0, scale: 1 }}
                        exit={{ opacity: 0, y: -8, scale: 0.97 }}
                        transition={{ duration: 0.12, ease: "easeOut" }}
                        className="absolute z-50 w-full mt-1 bg-surface-elevated border border-border rounded-lg shadow-xl overflow-hidden"
                    >
                        <div ref={listRef} className="overflow-y-auto custom-scrollbar py-1 max-h-56">
                            {options.map((option) => (
                                <button
                                    key={option.value}
                                    data-selected={option.value === value}
                                    onClick={() => handleSelect(option.value)}
                                    className={`w-full flex items-center justify-between px-3 py-2 text-sm text-left hover:bg-surface-hover transition-colors
                                        ${option.value === value ? 'text-accent bg-accent/5 font-medium' : 'text-text-primary'}`}
                                >
                                    <span className="flex items-center gap-2">
                                        {option.icon}
                                        {option.label}
                                    </span>
                                    {option.value === value && <Check size={14} />}
                                </button>
                            ))}
                        </div>
                    </motion.div>
                )}
            </AnimatePresence>
        </div>
    );
}
