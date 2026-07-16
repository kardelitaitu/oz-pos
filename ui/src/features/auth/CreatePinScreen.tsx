import { useState } from 'react';
import { useToast } from '@/frontend/shared/Toast';
import { bootstrapOwner } from '@/api/staff';
import { useAuth } from '@/contexts/AuthContext';
import './CreatePinScreen.css';

/** Props for the CreatePinScreen component. */
export interface CreatePinScreenProps {
  /** Callback invoked after the first owner is created and auto-logged in. */
  onCreated: () => void;
}

/** Create owner PIN screen — bootstraps the first owner account on a fresh installation. */
export default function CreatePinScreen({ onCreated }: CreatePinScreenProps) {
  const [displayName, setDisplayName] = useState('');
  const [username, setUsername] = useState('');
  const [pin, setPin] = useState('');
  const [confirmPin, setConfirmPin] = useState('');
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const { addToast } = useToast();
  const { swapSession } = useAuth();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrorMsg(null);

    if (!displayName.trim() || !username.trim() || !pin.trim()) {
      setErrorMsg('All fields are required.');
      return;
    }
    if (pin.length < 4) {
      setErrorMsg('PIN must be at least 4 characters.');
      return;
    }
    if (pin !== confirmPin) {
      setErrorMsg('PINs do not match.');
      return;
    }

    setLoading(true);
    try {
      const result = await bootstrapOwner({
        username: username.trim(),
        pin,
        display_name: displayName.trim(),
      });
      swapSession(result.session);
      addToast({ type: 'success', message: 'Owner account created successfully!' });
      onCreated();
      } catch (err: unknown) {
        let message = 'An error occurred while creating the owner account.';
        if (err instanceof Error) message = err.message;
        else if (typeof err === 'string') message = err;
        else if (err && typeof err === 'object' && 'message' in err) {
          message = String((err as Record<string, unknown>)['message']);
        }
        // Users already exist — someone else set up already, go to login.
        if (message.toLowerCase().includes('already exist')) {
          onCreated();
          return;
        }
        setErrorMsg(message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="create-pin-container">
      <div className="create-pin-card">
        <div className="create-pin-header">
          <h1>Create Owner PIN</h1>
          <p>Set up the first owner account to manage your POS</p>
        </div>

        {errorMsg && (
          <div className="create-pin-error-banner" role="alert">
            {errorMsg}
          </div>
        )}

        <form onSubmit={handleSubmit}>
          <div className="create-pin-form-group">
            <label htmlFor="displayName">Display Name</label>
            <input
              id="displayName"
              type="text"
              className="create-pin-input"
              placeholder="Store Owner"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              disabled={loading}
            />
          </div>

          <div className="create-pin-form-group">
            <label htmlFor="username">Username</label>
            <input
              id="username"
              type="text"
              className="create-pin-input"
              placeholder="owner"
              value={username}
              onChange={(e) => setUsername(e.target.value.toLowerCase())}
              disabled={loading}
              autoComplete="username"
            />
          </div>

          <div className="create-pin-form-group">
            <label htmlFor="pin">PIN</label>
            <input
              id="pin"
              type="password"
              className="create-pin-input"
              placeholder="At least 4 digits"
              value={pin}
              onChange={(e) => setPin(e.target.value)}
              disabled={loading}
              autoComplete="new-password"
              inputMode="numeric"
              maxLength={8}
            />
          </div>

          <div className="create-pin-form-group">
            <label htmlFor="confirmPin">Confirm PIN</label>
            <input
              id="confirmPin"
              type="password"
              className="create-pin-input"
              placeholder="Re-enter PIN"
              value={confirmPin}
              onChange={(e) => setConfirmPin(e.target.value)}
              disabled={loading}
              autoComplete="new-password"
              inputMode="numeric"
              maxLength={8}
            />
          </div>

          <button
            type="submit"
            className="create-pin-submit-btn"
            disabled={loading || !displayName || !username || !pin || !confirmPin}
          >
            {loading ? (
              <>
                <svg className="spinner" viewBox="0 0 24 24" width="20" height="20" stroke="currentColor" strokeWidth="2" fill="none">
                  <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
                  <path d="M12 2a10 10 0 0 1 10 10" />
                </svg>
                Creating...
              </>
            ) : (
              'Create Owner Account'
            )}
          </button>
        </form>
      </div>
    </div>
  );
}
