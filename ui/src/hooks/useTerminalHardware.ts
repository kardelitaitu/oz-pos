import { useState, useEffect, useCallback, useRef } from 'react';
import { getHardwareSettings, setHardwareSettingsScoped, type HardwareSettingsDto } from '@/api/settings';

// ── Types ──────────────────────────────────────────────────────────

export type PrinterConnection = 'network' | 'usb' | 'serial' | 'auto';
export type PaperSize = '58' | '80' | 'a4' | 'letter';
export type ScaleConnection = 'serial' | 'usb' | 'none';
export type ScannerMode = 'keyboard' | 'serial' | 'auto';

export interface PrinterConfig {
  connection: PrinterConnection;
  devicePath: string;
  paperSize: PaperSize;
  testPrintIp: string;
}

export interface ScaleConfig {
  connection: ScaleConnection;
  devicePath: string;
  baudRate: number;
  zeroOnBoot: boolean;
}

export interface ScannerConfig {
  mode: ScannerMode;
  deviceId: string;
}

export interface HardwareConfig {
  printer: PrinterConfig;
  scale: ScaleConfig;
  scanner: ScannerConfig;
}

export interface LocalPrefs {
  soundVolume: number;
  darkMode: boolean;
  scaleAutoZero: boolean;
}

export interface TerminalHardwareProfile {
  terminalId: string;
  storeId: string;
  hardware: HardwareConfig;
  localPrefs: LocalPrefs;
  initialized: string;
  version: number;
}

// ── Defaults ────────────────────────────────────────────────────────

const DEFAULT_PRINTER: PrinterConfig = {
  connection: 'auto',
  devicePath: '',
  paperSize: '80',
  testPrintIp: '',
};

const DEFAULT_SCALE: ScaleConfig = {
  connection: 'none',
  devicePath: '',
  baudRate: 9600,
  zeroOnBoot: false,
};

const DEFAULT_SCANNER: ScannerConfig = {
  mode: 'auto',
  deviceId: '',
};

const DEFAULT_LOCAL_PREFS: LocalPrefs = {
  soundVolume: 80,
  darkMode: false,
  scaleAutoZero: true,
};

/** Create a default profile for a given terminal. */
export function createDefaultProfile(terminalId: string, storeId?: string): TerminalHardwareProfile {
  return {
    terminalId,
    storeId: storeId ?? '',
    hardware: {
      printer: { ...DEFAULT_PRINTER },
      scale: { ...DEFAULT_SCALE },
      scanner: { ...DEFAULT_SCANNER },
    },
    localPrefs: { ...DEFAULT_LOCAL_PREFS },
    initialized: new Date().toISOString(),
    version: 1,
  };
}

// ── Storage key ─────────────────────────────────────────────────────

function storageKey(terminalId: string): string {
  return `oz-pos-terminal-hardware-${terminalId}`;
}

// TODO (Phase 0c): When Tauri fs plugin is integrated, replace
// localStorage with filesystem reads/writes to:
//   %APPDATA%/oz-pos/terminals/{terminalId}/terminal_profile.json (Windows)
//   ~/.local/share/oz-pos/terminals/{terminalId}/terminal_profile.json (Linux/macOS)

// ── Validation ──────────────────────────────────────────────────────

interface ValidationResult {
  valid: boolean;
  errors: string[];
}

/** Validate a parsed object against the TerminalHardwareProfile schema.
 *  Returns a list of field-level errors. Invalid fields are reset to
 *  defaults by the caller via mergeWithDefaults. */
function validateProfile(obj: unknown): ValidationResult {
  const errors: string[] = [];

  if (!obj || typeof obj !== 'object') {
    return { valid: false, errors: ['Profile is not an object'] };
  }

  const p = obj as Record<string, unknown>;

  if (typeof p['terminalId'] !== 'string') errors.push('terminalId: expected string');
  if (typeof p['storeId'] !== 'string') errors.push('storeId: expected string');
  if (typeof p['version'] !== 'number') errors.push('version: expected number');
  if (typeof p['initialized'] !== 'string') errors.push('initialized: expected string');

  // hardware
  if (p['hardware'] && typeof p['hardware'] === 'object') {
    const hw = p['hardware'] as Record<string, unknown>;

    if (hw['printer'] && typeof hw['printer'] === 'object') {
      const pr = hw['printer'] as Record<string, unknown>;
      if (pr['connection'] && !['network', 'usb', 'serial', 'auto'].includes(String(pr['connection']))) {
        errors.push('hardware.printer.connection: invalid value');
      }
      if (pr['paperSize'] && !['58', '80', 'a4', 'letter'].includes(String(pr['paperSize']))) {
        errors.push('hardware.printer.paperSize: invalid value');
      }
    }

    if (hw['scale'] && typeof hw['scale'] === 'object') {
      const sc = hw['scale'] as Record<string, unknown>;
      if (sc['connection'] && !['serial', 'usb', 'none'].includes(String(sc['connection']))) {
        errors.push('hardware.scale.connection: invalid value');
      }
      if (typeof sc['baudRate'] === 'number' && (sc['baudRate'] as number) < 0) {
        errors.push('hardware.scale.baudRate: must be non-negative');
      }
    }

    if (hw['scanner'] && typeof hw['scanner'] === 'object') {
      const sn = hw['scanner'] as Record<string, unknown>;
      if (sn['mode'] && !['keyboard', 'serial', 'auto'].includes(String(sn['mode']))) {
        errors.push('hardware.scanner.mode: invalid value');
      }
    }
  } else {
    errors.push('hardware: expected object');
  }

  // localPrefs
  if (p['localPrefs'] && typeof p['localPrefs'] === 'object') {
    const lp = p['localPrefs'] as Record<string, unknown>;
    if (typeof lp['soundVolume'] === 'number' && ((lp['soundVolume'] as number) < 0 || (lp['soundVolume'] as number) > 100)) {
      errors.push('localPrefs.soundVolume: must be 0-100');
    }
    if (lp['darkMode'] !== undefined && typeof lp['darkMode'] !== 'boolean') {
      errors.push('localPrefs.darkMode: expected boolean');
    }
    if (lp['scaleAutoZero'] !== undefined && typeof lp['scaleAutoZero'] !== 'boolean') {
      errors.push('localPrefs.scaleAutoZero: expected boolean');
    }
  } else {
    errors.push('localPrefs: expected object');
  }

  return { valid: errors.length === 0, errors };
}

const VALID_PRINTER_CONNS: PrinterConnection[] = ['network', 'usb', 'serial', 'auto'];
const VALID_PAPER_SIZES: PaperSize[] = ['58', '80', 'a4', 'letter'];
const VALID_SCALE_CONNS: ScaleConnection[] = ['serial', 'usb', 'none'];
const VALID_SCANNER_MODES: ScannerMode[] = ['keyboard', 'serial', 'auto'];

/** Merge a partially-valid profile with defaults, resetting invalid enum fields. */
function mergeWithDefaults(partial: TerminalHardwareProfile): TerminalHardwareProfile {
  const defaults = createDefaultProfile(partial.terminalId, partial.storeId);
  const result = { ...defaults };

  const pp = partial.hardware?.printer;
  result.hardware.printer = {
    connection: pp?.connection && VALID_PRINTER_CONNS.includes(pp.connection as PrinterConnection)
      ? pp.connection : defaults.hardware.printer.connection,
    devicePath: pp?.devicePath ?? defaults.hardware.printer.devicePath,
    paperSize: pp?.paperSize && VALID_PAPER_SIZES.includes(pp.paperSize as PaperSize)
      ? pp.paperSize : defaults.hardware.printer.paperSize,
    testPrintIp: pp?.testPrintIp ?? defaults.hardware.printer.testPrintIp,
  };

  const sp = partial.hardware?.scale;
  result.hardware.scale = {
    connection: sp?.connection && VALID_SCALE_CONNS.includes(sp.connection as ScaleConnection)
      ? sp.connection : defaults.hardware.scale.connection,
    devicePath: sp?.devicePath ?? defaults.hardware.scale.devicePath,
    baudRate: typeof sp?.baudRate === 'number' && sp.baudRate >= 0
      ? sp.baudRate : defaults.hardware.scale.baudRate,
    zeroOnBoot: typeof sp?.zeroOnBoot === 'boolean' ? sp.zeroOnBoot : defaults.hardware.scale.zeroOnBoot,
  };

  const snp = partial.hardware?.scanner;
  result.hardware.scanner = {
    mode: snp?.mode && VALID_SCANNER_MODES.includes(snp.mode as ScannerMode)
      ? snp.mode : defaults.hardware.scanner.mode,
    deviceId: snp?.deviceId ?? defaults.hardware.scanner.deviceId,
  };

  const lp = partial.localPrefs;
  result.localPrefs = {
    soundVolume: typeof lp?.soundVolume === 'number' && lp.soundVolume >= 0 && lp.soundVolume <= 100
      ? lp.soundVolume : defaults.localPrefs.soundVolume,
    darkMode: typeof lp?.darkMode === 'boolean' ? lp.darkMode : defaults.localPrefs.darkMode,
    scaleAutoZero: typeof lp?.scaleAutoZero === 'boolean' ? lp.scaleAutoZero : defaults.localPrefs.scaleAutoZero,
  };

  if (partial.initialized) result.initialized = partial.initialized;
  result.version = Math.max(defaults.version, partial.version ?? 0);

  return result;
}

// ── Bridge: sync printer/scanner subset to Tauri IPC (filesystem backend) ──

/** Extract the IPC-compatible DTO subset from a full profile. */
function toHardwareSettingsDto(profile: TerminalHardwareProfile): HardwareSettingsDto {
  return {
    printerConnection: profile.hardware.printer.connection,
    printerDevicePath: profile.hardware.printer.devicePath,
    printerPaperSize: profile.hardware.printer.paperSize,
    scannerDeviceId: profile.hardware.scanner.deviceId,
    scannerInputMode: profile.hardware.scanner.mode,
  };
}

/** Merge IPC DTO fields into a profile — IPC values take precedence when present. */
function mergeIpcSettings(
  profile: TerminalHardwareProfile,
  dto: HardwareSettingsDto,
): TerminalHardwareProfile {
  return {
    ...profile,
    hardware: {
      ...profile.hardware,
      printer: {
        ...profile.hardware.printer,
        connection: dto.printerConnection
          ? (dto.printerConnection as PrinterConnection)
          : profile.hardware.printer.connection,
        devicePath: dto.printerDevicePath ?? profile.hardware.printer.devicePath,
        paperSize: dto.printerPaperSize
          ? (dto.printerPaperSize as PaperSize)
          : profile.hardware.printer.paperSize,
      },
      scanner: {
        ...profile.hardware.scanner,
        deviceId: dto.scannerDeviceId ?? profile.hardware.scanner.deviceId,
        mode: dto.scannerInputMode
          ? (dto.scannerInputMode as ScannerMode)
          : profile.hardware.scanner.mode,
      },
    },
  };
}

// ── Read from storage ───────────────────────────────────────────────

function readFromStorage(terminalId: string): string | null {
  return localStorage.getItem(storageKey(terminalId));
}

function writeToStorage(terminalId: string, data: string): void {
  localStorage.setItem(storageKey(terminalId), data);
}

function removeFromStorage(terminalId: string): void {
  localStorage.removeItem(storageKey(terminalId));
}

// ── Hook return type ────────────────────────────────────────────────

export interface UseTerminalHardwareResult {
  /** The current hardware profile (never null after initial load). */
  profile: TerminalHardwareProfile | null;
  /** True during initial load or save. */
  isLoading: boolean;
  /** Error from the most recent operation, or null. */
  error: string | null;
  /** Update printer configuration (local state only, call save() to persist). */
  updatePrinter: (partial: Partial<PrinterConfig>) => void;
  /** Update scale configuration. */
  updateScale: (partial: Partial<ScaleConfig>) => void;
  /** Update scanner configuration. */
  updateScanner: (partial: Partial<ScannerConfig>) => void;
  /** Update local preferences. */
  updateLocalPrefs: (partial: Partial<LocalPrefs>) => void;
  /** Persist the current profile to storage (three-phase commit). */
  save: (sessionToken?: string) => Promise<void>;
  /** Re-read profile from storage. */
  reload: () => void;
}

// ── Hook ────────────────────────────────────────────────────────────

/**
 * Hook to manage terminal hardware bindings (printer, scale, scanner)
 * stored per-terminal in localStorage (filesystem when Tauri fs plugin
 * is available in future).
 *
 * @param terminalId - Unique terminal identifier
 * @param storeId - Optional store identifier for the profile
 */
export function useTerminalHardware(
  terminalId: string,
  storeId?: string,
): UseTerminalHardwareResult {
  const [profile, setProfile] = useState<TerminalHardwareProfile | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const initializedRef = useRef(false);

  // ── Load profile on mount ──────────────────────────────────

  const loadProfile = useCallback(async () => {
    if (!terminalId) {
      setProfile(null);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const raw = readFromStorage(terminalId);
      let profileData: TerminalHardwareProfile;

      if (!raw) {
        profileData = createDefaultProfile(terminalId, storeId);
      } else {
        let parsed: unknown;
        try {
          parsed = JSON.parse(raw);
        } catch {
          handleCorruptedFile(terminalId, raw);
          profileData = createDefaultProfile(terminalId, storeId);
          setProfile(profileData);
          setIsLoading(false);
          setError('Hardware profile was corrupted — reset to defaults.');
          return;
        }

        const validation = validateProfile(parsed);
        if (!validation.valid) {
          profileData = mergeWithDefaults(parsed as TerminalHardwareProfile);
          try {
            writeToStorage(terminalId, JSON.stringify(profileData, null, 2));
          } catch { /* best-effort */ }
        } else {
          profileData = parsed as TerminalHardwareProfile;
        }
      }

      // Bridge: merge IPC (filesystem) printer/scanner values on top of localStorage
      try {
        const ipcSettings = await getHardwareSettings();
        profileData = mergeIpcSettings(profileData, ipcSettings);
      } catch {
        // IPC unavailable (dev mock, non-Tauri env) — use localStorage only
      }

      setProfile(profileData);
      setIsLoading(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load terminal hardware profile');
      setIsLoading(false);
    }
  }, [terminalId, storeId]);

  useEffect(() => {
    if (!initializedRef.current || terminalId !== profile?.terminalId) {
      initializedRef.current = true;
      loadProfile();
    }
  }, [terminalId, loadProfile, profile?.terminalId]);

  // ── Update helpers (local state only) ───────────────────────

  const updatePrinter = useCallback((partial: Partial<PrinterConfig>) => {
    setProfile((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        hardware: {
          ...prev.hardware,
          printer: { ...prev.hardware.printer, ...partial },
        },
      };
    });
  }, []);

  const updateScale = useCallback((partial: Partial<ScaleConfig>) => {
    setProfile((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        hardware: {
          ...prev.hardware,
          scale: { ...prev.hardware.scale, ...partial },
        },
      };
    });
  }, []);

  const updateScanner = useCallback((partial: Partial<ScannerConfig>) => {
    setProfile((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        hardware: {
          ...prev.hardware,
          scanner: { ...prev.hardware.scanner, ...partial },
        },
      };
    });
  }, []);

  const updateLocalPrefs = useCallback((partial: Partial<LocalPrefs>) => {
    setProfile((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        localPrefs: { ...prev.localPrefs, ...partial },
      };
    });
  }, []);

  // ── Three-phase commit save (localStorage + IPC bridge) ─────

  const save = useCallback(async (sessionToken?: string) => {
    if (!profile || !terminalId) return;

    setIsLoading(true);
    setError(null);

    const newData = JSON.stringify(profile, null, 2);
    const oldData = readFromStorage(terminalId);

    try {
      // Phase 1: Write to localStorage (backward compatible)
      writeToStorage(terminalId, newData);

      // Phase 2: Bridge printer/scanner subset to IPC (filesystem backend)
      if (sessionToken) {
        try {
          await setHardwareSettingsScoped(sessionToken, toHardwareSettingsDto(profile));
        } catch {
          // IPC unavailable — localStorage-only is fine
        }
      }

      setIsLoading(false);
    } catch (err) {
      // Failure: Restore from old data
      if (oldData !== null) {
        try { writeToStorage(terminalId, oldData); } catch { /* desperate */ }
      } else {
        removeFromStorage(terminalId);
      }
      const msg = err instanceof Error ? err.message : 'Failed to save hardware profile';
      setError(msg);
      setIsLoading(false);
    }
  }, [profile, terminalId]);

  // ── Reload ──────────────────────────────────────────────────

  const reload = useCallback(() => {
    loadProfile();
  }, [loadProfile]);

  return {
    profile,
    isLoading,
    error,
    updatePrinter,
    updateScale,
    updateScanner,
    updateLocalPrefs,
    save,
    reload,
  };
}

// ── Corruption recovery ─────────────────────────────────────────────

/** Handle a corrupted profile file: rename it, create fresh defaults. */
function handleCorruptedFile(terminalId: string, corruptedRaw: string): void {
  const key = storageKey(terminalId);
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
  const corruptedKey = `${key}.corrupted.${timestamp}`;

  try {
    // Save the corrupted data for forensic purposes
    localStorage.setItem(corruptedKey, corruptedRaw);
    // Clear the original key so a fresh profile is created
    localStorage.removeItem(key);
  } catch {
    // If even the backup fails, just clear the original
    try {
      localStorage.removeItem(key);
    } catch { /* hopeless */ }
  }
}
