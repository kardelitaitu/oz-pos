import React, { useState, useEffect } from 'react';
import SearchModal from './SearchModal';

interface Props {
  locale: string;
}

export default function SearchTrigger({ locale }: Props) {
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setIsOpen((prev) => !prev);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <>
      <button
        type="button"
        onClick={() => setIsOpen(true)}
        aria-label="Search"
        className="flex items-center gap-2 rounded-md border border-ink/10 bg-surface/60 px-2.5 py-1.5 text-xs text-muted transition hover:border-ink/20 hover:text-ink"
      >
        <svg className="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>
        <span className="hidden sm:inline">{locale === 'id' ? 'Cari…' : 'Search…'}</span>
        <kbd className="hidden sm:inline-block rounded border border-ink/15 bg-ink/5 px-1 py-0.2 text-[10px] text-muted font-mono">
          ⌘K
        </kbd>
      </button>
      <SearchModal isOpen={isOpen} onClose={() => setIsOpen(false)} locale={locale} />
    </>
  );
}
