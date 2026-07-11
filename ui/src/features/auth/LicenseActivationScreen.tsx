import { useState } from 'react';
import { useToast } from '@/frontend/shared/Toast';
import { activateLicense } from '@/api/license';
import './LicenseActivationScreen.css';

export interface LicenseActivationScreenProps {
  onActivated: () => void;
}

export default function LicenseActivationScreen({ onActivated }: LicenseActivationScreenProps) {
  const [key, setKey] = useState('');
  const [tenantId, setTenantId] = useState('');
  const [loading, setLoading] = useState(false);
  const { addToast } = useToast();

  const handleActivate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!key.trim() || !tenantId.trim()) {
      addToast({ type: 'error', message: 'License key and Tenant ID are required.' });
      return;
    }

    setLoading(true);
    try {
      // In a real app we might fetch machineId using a hardware profile API, 
      // but for now we generate a random one or use a placeholder.
      const machineId = 'MACH-' + Math.random().toString(36).substr(2, 9).toUpperCase();

      const success = await activateLicense(
        key.trim(),
        tenantId.trim(),
        machineId
      );

      if (success) {
        addToast({ type: 'success', message: 'License activated successfully!' });
        onActivated();
      } else {
        addToast({ type: 'error', message: 'Failed to activate license.' });
      }
    } catch (err: any) {
      addToast({ 
        type: 'error', 
        message: err.message || 'An error occurred during activation.' 
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

        <form onSubmit={handleActivate}>
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
              autoFocus
            />
          </div>

          <div className="license-form-group">
            <label htmlFor="tenantId">Tenant / Store ID</label>
            <input
              id="tenantId"
              type="text"
              className="license-input"
              placeholder="e.g. Store 1 or Main Register"
              value={tenantId}
              onChange={(e) => setTenantId(e.target.value)}
              disabled={loading}
            />
          </div>

          <button 
            type="submit" 
            className="license-submit-btn" 
            disabled={loading || !key || !tenantId}
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
    </div>
  );
}
