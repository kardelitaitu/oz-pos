import React, { useState, useEffect, useRef, useMemo } from 'react';
import { createPortal } from 'react-dom';
import { t } from '../i18n';

export interface SearchItem {
  id: string;
  title: string;
  category: 'docs' | 'pages';
  url: string;
  keywords?: string;
}

interface Props {
  isOpen: boolean;
  onClose: () => void;
  locale: string;
}

export default function SearchModal({ isOpen, onClose, locale }: Props) {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const dialogRef = useRef<HTMLDivElement>(null);
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const searchItems: SearchItem[] = useMemo(
    () => [
      // Core pages
      { id: 'home', title: locale === 'id' ? 'Beranda' : 'Home', category: 'pages', url: `/${locale}` },
      { id: 'pricing', title: locale === 'id' ? 'Harga & Paket' : 'Pricing & Plans', category: 'pages', url: `/${locale}/pricing`, keywords: 'plans subscription pro plus free enterprise cost' },
      { id: 'download', title: locale === 'id' ? 'Unduh Aplikasi' : 'Download Application', category: 'pages', url: `/${locale}/download`, keywords: 'windows macos linux android ios tablet pos terminal installer' },
      { id: 'features', title: locale === 'id' ? 'Fitur Lengkap' : 'Features & Architecture', category: 'pages', url: `/${locale}/features`, keywords: 'offline kds multi store shifts inventory payments' },
      { id: 'account', title: locale === 'id' ? 'Dashboard Akun & Lisensi' : 'Account & License Dashboard', category: 'pages', url: `/${locale}/account`, keywords: 'profile subscription license terminals' },
      { id: 'support', title: locale === 'id' ? 'Bantuan & Kontak' : 'Support & Contact', category: 'pages', url: `/${locale}/support`, keywords: 'faq contact discord email help' },
      
      // Vertical Solutions
      { id: 'kafe', title: locale === 'id' ? 'POS untuk Kafe & Kedai Kopi' : 'POS for Cafes & Coffee Shops', category: 'pages', url: `/${locale}/untuk-kafe`, keywords: 'cafe coffee table orders kds modifiers' },
      { id: 'restoran', title: locale === 'id' ? 'POS untuk Restoran & F&B' : 'POS for Restaurants', category: 'pages', url: `/${locale}/untuk-restoran`, keywords: 'restaurant kitchen display split bill service charge' },
      { id: 'minimarket', title: locale === 'id' ? 'POS untuk Minimarket & Retail' : 'POS for Minimarkets & Retail', category: 'pages', url: `/${locale}/untuk-minimarket`, keywords: 'barcode scanning sku inventory fast retail' },
      { id: 'warung', title: locale === 'id' ? 'POS untuk Warung & UMKM' : 'POS for Warung & Small Business', category: 'pages', url: `/${locale}/untuk-warung`, keywords: 'umkm warung simple affordable fast cash qris' },

      // Documentation
      { id: 'doc-welcome', title: locale === 'id' ? 'Pengenalan OZ-POS' : 'Welcome to OZ-POS', category: 'docs', url: `/${locale}/docs/welcome`, keywords: 'getting started overview architecture introduction' },
      { id: 'doc-activation', title: locale === 'id' ? 'Aktivasi Lisensi & Terminal' : 'License & Terminal Activation', category: 'docs', url: `/${locale}/docs/activation`, keywords: 'activate license key register terminal offline token' },
      { id: 'doc-installation', title: locale === 'id' ? 'Panduan Instalasi' : 'Installation Guide', category: 'docs', url: `/${locale}/docs/installation`, keywords: 'install desktop windows linux macos build' },
      { id: 'doc-first-sale', title: locale === 'id' ? 'Membuat Transaksi Pertama' : 'Processing Your First Sale', category: 'docs', url: `/${locale}/docs/first-sale`, keywords: 'pos checkout cash card barcode print receipt' },
      { id: 'doc-inventory', title: locale === 'id' ? 'Manajemen Stok & SKU' : 'Inventory & Stock Management', category: 'docs', url: `/${locale}/docs/inventory`, keywords: 'stock items inventory variants low stock alert' },
      { id: 'doc-payments', title: locale === 'id' ? 'Integrasi Pembayaran & QRIS' : 'Payments & QRIS Integration', category: 'docs', url: `/${locale}/docs/payments`, keywords: 'midtrans paddle qris card edc cash payments' },
      { id: 'doc-shifts', title: locale === 'id' ? 'Manajemen Shift & Kasir' : 'Shift Management & Cash Drawer', category: 'docs', url: `/${locale}/docs/shifts`, keywords: 'cash in cash out shift end float reconciliation' },
      { id: 'doc-cloud-sync', title: locale === 'id' ? 'Sinkronisasi Cloud & Offline' : 'Cloud Sync & Offline Mode', category: 'docs', url: `/${locale}/docs/cloud-sync`, keywords: 'offline local first peer to peer sync cloud backup' },
    ],
    [locale]
  );

  const filteredItems = useMemo(() => {
    if (!query.trim()) return searchItems.slice(0, 8);
    const q = query.trim().toLowerCase();
    return searchItems.filter(
      (item) =>
        item.title.toLowerCase().includes(q) ||
        item.url.toLowerCase().includes(q) ||
        item.keywords?.toLowerCase().includes(q)
    );
  }, [query, searchItems]);

  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isOpen) {
        if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
          e.preventDefault();
          // Can be toggled if global handler is bound
        }
        return;
      }

      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev < filteredItems.length - 1 ? prev + 1 : 0));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : filteredItems.length - 1));
      } else if (e.key === 'Enter' && filteredItems[selectedIndex]) {
        e.preventDefault();
        window.location.href = filteredItems[selectedIndex].url;
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose, filteredItems, selectedIndex]);

  useEffect(() => {
    if (!isOpen) return;
    const handleMouseDown = (e: MouseEvent) => {
      if (dialogRef.current && !dialogRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener('mousedown', handleMouseDown);
    return () => document.removeEventListener('mousedown', handleMouseDown);
  }, [isOpen, onClose]);

  if (!isOpen || !mounted) return null;

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-16 px-4 sm:pt-24"
      onClick={onClose}
    >
      {/* Backdrop */}
      <div
        data-backdrop="true"
        onClick={onClose}
        className="fixed inset-0 bg-ink/40 backdrop-blur-sm transition-opacity"
        aria-hidden="true"
      />

      {/* Modal Dialog */}
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-label={t(locale, 'search.placeholder')}
        className="relative z-10 w-full max-w-lg rounded-2xl border border-ink/15 bg-surface p-4 shadow-2xl transition-all"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Search Bar Input */}
        <div className="relative flex items-center border-b border-ink/10 pb-3">
          <svg className="w-5 h-5 text-muted ml-1 mr-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <input
            ref={inputRef}
            type="search"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            placeholder={t(locale, 'search.placeholder')}
            className="w-full bg-transparent text-sm text-ink outline-none placeholder:text-muted"
            autoComplete="off"
            spellCheck="false"
          />
          <kbd className="hidden sm:inline-block rounded border border-ink/15 bg-ink/5 px-1.5 py-0.5 text-xs text-muted">
            ESC
          </kbd>
        </div>

        {/* Search Results List */}
        <div className="mt-3 max-h-80 overflow-y-auto space-y-1">
          {filteredItems.length === 0 ? (
            <div className="py-8 text-center text-sm text-muted">
              {t(locale, 'search.noResults')} <span className="font-semibold text-ink">"{query}"</span>
            </div>
          ) : (
            filteredItems.map((item, idx) => (
              <a
                key={item.id}
                role="option"
                aria-selected={selectedIndex === idx}
                href={item.url}
                onMouseEnter={() => setSelectedIndex(idx)}
                className={`flex items-center justify-between rounded-lg px-3 py-2 text-sm transition ${
                  selectedIndex === idx
                    ? 'bg-accent/15 text-link'
                    : 'text-ink hover:bg-ink/5'
                }`}
              >
                <div className="flex items-center gap-2.5">
                  <span className="text-muted">
                    {item.category === 'docs' ? (
                      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
                        <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
                      </svg>
                    ) : (
                      <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                        <polygon points="12 2 2 7 12 12 22 7 12 2" />
                        <polyline points="2 17 12 22 22 17" />
                        <polyline points="2 12 12 17 22 12" />
                      </svg>
                    )}
                  </span>
                  <span className="font-medium">{item.title}</span>
                </div>
                <span className="text-xs uppercase tracking-wider text-muted font-mono">
                  {item.category}
                </span>
              </a>
            ))
          )}
        </div>

        {/* Footer Shortcut Helper */}
        <div className="mt-3 border-t border-ink/10 pt-2 text-center text-xs text-muted">
          {t(locale, 'search.shortcutHint')}
        </div>
      </div>
    </div>,
    document.body
  );
}
