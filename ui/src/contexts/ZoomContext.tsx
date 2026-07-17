import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';

/** Application zoom level preset. */
export type ZoomLevel = 'auto' | '100' | '125' | '150' | '200';

interface ZoomContextType {
  zoomLevel: ZoomLevel;
  setZoomLevel: (level: ZoomLevel) => void;
}

const ZoomContext = createContext<ZoomContextType | undefined>(undefined);

/**
 * Provides zoom/scaling state to the application.
 * Adjusts the root `font-size` based on the selected zoom level.
 * Intercepts Ctrl+/-/0 for manual zoom control and persists the
 * choice to localStorage.
 */
export function ZoomProvider({ children }: { children: ReactNode }) {
  const [zoomLevel, setZoomLevel] = useState<ZoomLevel>(() => {
    const saved = localStorage.getItem('app-zoom-level');
    return (saved as ZoomLevel) || 'auto';
  });

  useEffect(() => {
    localStorage.setItem('app-zoom-level', zoomLevel);

    const applyZoom = () => {
      let fontSize = 16;
      if (zoomLevel === 'auto') {
        const windowWidth = window.innerWidth;
        // Base resolution is 1920px = 16px base font.
        // Scale DOWN below 1920px but never UP above it. Tauri/WebView2
        // already respects Windows display scaling (125%, 150%, etc.) so
        // additional width-based up-scaling would double-scale on 4K.
        const scale = windowWidth / 1920;
        // Clamp between 14px (minimum readable at 1366x768) and 16px (base)
        fontSize = Math.max(14, Math.min(16, 16 * scale));
      } else {
        const percentage = parseInt(zoomLevel, 10);
        fontSize = 16 * (percentage / 100);
      }
      
      document.documentElement.style.fontSize = `${fontSize}px`;
    };

    applyZoom();

    // Re-apply if screen resolution changes
    window.addEventListener('resize', applyZoom);

    // Intercept Ctrl +/- / 0 to handle zooming manually
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey) {
        if (e.key === '=' || e.key === '+' || e.code === 'NumpadAdd') {
          e.preventDefault();
          setZoomLevel((prev) => {
            if (prev === 'auto') return '125';
            if (prev === '100') return '125';
            if (prev === '125') return '150';
            if (prev === '150') return '200';
            return '200';
          });
        } else if (e.key === '-' || e.code === 'NumpadSubtract') {
          e.preventDefault();
          setZoomLevel((prev) => {
            if (prev === 'auto') return '100';
            if (prev === '200') return '150';
            if (prev === '150') return '125';
            if (prev === '125') return '100';
            return '100';
          });
        } else if (e.key === '0' || e.code === 'Numpad0') {
          e.preventDefault();
          setZoomLevel('auto');
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('resize', applyZoom);
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [zoomLevel]);

  return (
    <ZoomContext.Provider value={{ zoomLevel, setZoomLevel }}>
      {children}
    </ZoomContext.Provider>
  );
}

/** Access the zoom context. Must be used within a `<ZoomProvider>`. */
// eslint-disable-next-line react-refresh/only-export-components
export function useAppZoom() {
  const context = useContext(ZoomContext);
  if (context === undefined) {
    throw new Error('useAppZoom must be used within a ZoomProvider');
  }
  return context;
}
