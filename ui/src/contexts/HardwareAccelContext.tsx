import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import './HardwareAccel.css';

interface HardwareAccelContextType {
  /** Whether CSS GPU acceleration hints (will-change, translateZ, backdrop-filter) are enabled. */
  enabled: boolean;
  /** Toggle hardware acceleration on/off. */
  setEnabled: (value: boolean) => void;
}

const HardwareAccelContext = createContext<HardwareAccelContextType | undefined>(undefined);

const STORAGE_KEY = 'app-hw-accel';

/**
 * Provides hardware acceleration state to the application.
 *
 * When disabled, sets `data-hw-accel="disabled"` on `<html>` so all CSS
 * `will-change`, `transform: translateZ(0)`, and `backdrop-filter` rules
 * can be overridden globally. Persists the choice to localStorage.
 */
export function HardwareAccelProvider({ children }: { children: ReactNode }) {
  const [enabled, setEnabled] = useState<boolean>(() => {
    const saved = localStorage.getItem(STORAGE_KEY);
    // Default to enabled (null = never saved = enabled).
    return saved !== 'false';
  });

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, String(enabled));

    if (enabled) {
      document.documentElement.removeAttribute('data-hw-accel');
    } else {
      document.documentElement.setAttribute('data-hw-accel', 'disabled');
    }
  }, [enabled]);

  return (
    <HardwareAccelContext.Provider value={{ enabled, setEnabled }}>
      {children}
    </HardwareAccelContext.Provider>
  );
}

// eslint-disable-next-line react-refresh/only-export-components
/** Access the hardware acceleration context. Must be used within a `<HardwareAccelProvider>`. */
export function useHardwareAccel() {
  const context = useContext(HardwareAccelContext);
  if (context === undefined) {
    throw new Error('useHardwareAccel must be used within a HardwareAccelProvider');
  }
  return context;
}
