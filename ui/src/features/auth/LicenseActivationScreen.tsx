import { useState } from 'react';
import { useToast } from '@/frontend/shared/Toast';
import { activateLicense, getMachineId } from '@/api/license';
import ConnectionStatus from '@/components/ConnectionStatus';
import './LicenseActivationScreen.css';

export interface LicenseActivationScreenProps {
  initialError?: string | null;
  onActivated: () => void;
}

export default function LicenseActivationScreen({ initialError, onActivated }: LicenseActivationScreenProps) {
  const [key, setKey] = useState('');
  const [email, setEmail] = useState('');
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(initialError ?? null);
  const { addToast } = useToast();

  const handleActivate = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMsg(null);
    if (!key.trim() || !email.trim()) {
      setErrorMsg('License key and Email are required.');
      return;
    }

    setLoading(true);
    try {
      // Get the persistent, cryptographically-generated machine ID from Rust.
      const machineId = await getMachineId();

      const success = await activateLicense(
        key.trim(),
        email.trim(),
        machineId
      );

      if (success) {
        addToast({ type: 'success', message: 'License activated successfully!' });
        onActivated();
      } else {
        setErrorMsg('Failed to activate license.');
      }
    } catch (err: unknown) {
      let message = 'An error occurred during activation.';
      if (err instanceof Error) {
        message = err.message;
      } else if (typeof err === 'string') {
        message = err;
      } else if (err && typeof err === 'object' && 'message' in err) {
        message = String((err as Record<string, unknown>).message);
      }
      
      addToast({ 
        type: 'error', 
        message,
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="license-activation-container">
      <div className="license-activation-card">
        <div className="license-activation-header">
          <h1>Activate OZ-POS</h1>
          <p>Enter your license key to unlock your terminal</p>
        </div>

        {errorMsg && (
          <div className="license-error-banner" style={{ background: 'rgba(239, 68, 68, 0.1)', color: '#ef4444', padding: '12px', borderRadius: '8px', marginBottom: '16px', fontSize: '14px' }}>
            {errorMsg}
          </div>
        )}

        <form onSubmit={handleActivate}>
          <div className="license-form-group">
            <label htmlFor="email">Email Address</label>
            <input
              id="email"
              type="email"
              className="license-input"
              placeholder="store@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              disabled={loading}
            />
          </div>

          <div className="license-form-group">
            <label htmlFor="licenseKey">License Key</label>
            <input
              id="licenseKey"
              type="text"
              className="license-input"
              placeholder="OZ-PRO-XXXX-XXXX-XXXX"
              value={key}
              onChange={(e) => setKey(e.target.value.toUpperCase())}
              disabled={loading}
            />
          </div>

          <button 
            type="submit" 
            className="license-submit-btn" 
            disabled={loading || !key || !email}
          >
            {loading ? (
              <>
                <svg className="spinner" viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" strokeWidth="2" fill="none">
                  <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
                  <path d="M12 2a10 10 0 0 1 10 10" />
                </svg>
                Activating...
              </>
            ) : (
              'Activate License'
            )}
          </button>
        </form>
      </div>

      <div className="license-server-status-container">
        <ConnectionStatus 
          label="Auth" 
          url="https://auth--oz-pos-license-service--76cyv4d6bn54.code.run" 
        />
        <ConnectionStatus 
          label="Sync" 
          url="" 
        />
      </div>
    </div>
  );
}
