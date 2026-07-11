import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';

export type ZoomLevel = 'auto' | '100' | '125' | '150' | '200';

interface ZoomContextType {
  zoomLevel: ZoomLevel;
  setZoomLevel: (level: ZoomLevel) => void;
}

const ZoomContext = createContext<ZoomContextType | undefined>(undefined);

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
        // Use window.outerWidth instead of innerWidth. 
        // This allows the app to adapt to window resizing without fighting the browser's native zoom (Ctrl +/-).
        const windowWidth = window.outerWidth || window.innerWidth;
        // Base resolution is 1920px (standard 1080p width) = 16px base font
        const scale = windowWidth / 1920;
        // Clamp between 14px (minimum readable) and 28px
        fontSize = Math.max(14, Math.min(28, 16 * scale));
      } else {
        const percentage = parseInt(zoomLevel, 10);
        fontSize = 16 * (percentage / 100);
      }
      
      document.documentElement.style.fontSize = `${fontSize}px`;
    };

    applyZoom();

    // Re-apply if screen resolution changes (e.g. moving window to a different monitor)
    window.addEventListener('resize', applyZoom);
    return () => window.removeEventListener('resize', applyZoom);
  }, [zoomLevel]);

  return (
    <ZoomContext.Provider value={{ zoomLevel, setZoomLevel }}>
      {children}
    </ZoomContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
export function useAppZoom() {
  const context = useContext(ZoomContext);
  if (context === undefined) {
    throw new Error('useAppZoom must be used within a ZoomProvider');
  }
  return context;
}
