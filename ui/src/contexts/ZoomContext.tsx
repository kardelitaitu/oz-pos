import React, { createContext, useContext, useEffect, useState } from 'react';

export type ZoomLevel = 'auto' | '100' | '125' | '150' | '200';

interface ZoomContextType {
  zoomLevel: ZoomLevel;
  setZoomLevel: (level: ZoomLevel) => void;
}

const ZoomContext = createContext<ZoomContextType | undefined>(undefined);

export function ZoomProvider({ children }: { children: React.ReactNode }) {
  const [zoomLevel, setZoomLevel] = useState<ZoomLevel>(() => {
    const saved = localStorage.getItem('app-zoom-level');
    return (saved as ZoomLevel) || 'auto';
  });

  useEffect(() => {
    localStorage.setItem('app-zoom-level', zoomLevel);

    const applyZoom = () => {
      let fontSize = 16;
      if (zoomLevel === 'auto') {
        // Use physical screen width so browser zoom (Ctrl +/-) isn't fought against
        const screenWidth = window.screen.width;
        // Base resolution 1366, scale proportionally
        const scale = screenWidth / 1366;
        // Clamp between 16px and 45px (4K)
        fontSize = Math.max(16, Math.min(45, 16 * scale));
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

export function useAppZoom() {
  const context = useContext(ZoomContext);
  if (context === undefined) {
    throw new Error('useAppZoom must be used within a ZoomProvider');
  }
  return context;
}
