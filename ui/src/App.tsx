import { useState } from 'react';
import { ThemeProvider } from '@/components/ThemeProvider';
import SetupWizard from '@/features/setup/SetupWizard';
import DesignSystem from '@/features/design/DesignSystem';
import '@/features/design/DesignSystem.css';

/**
 * Root app component.
 *
 * On first launch (no stored preset), the Setup Wizard is shown.
 * Once completed, the main Design System showcase is displayed.
 * This logic will be replaced with proper routing in Phase 2.
 */
export default function App() {
  return (
    <ThemeProvider>
      <AppShell />
    </ThemeProvider>
  );
}

function AppShell() {
  // Tracks whether the Setup Wizard has been completed or skipped.
  // In production this would read `store.setup_complete` from the
  // Tauri settings table via the IPC bridge.
  const [hasCompletedSetup, setHasCompletedSetup] = useState(false);

  const handleComplete = () => {
    setHasCompletedSetup(true);
    // Future: persist the WizardState via the Tauri IPC bridge
    // and set `store.setup_complete = "1"` in the settings table.
  };

  const handleSkip = () => {
    setHasCompletedSetup(true);
  };

  if (!hasCompletedSetup) {
    return (
      <SetupWizard onComplete={handleComplete} onSkip={handleSkip} />
    );
  }

  return <DesignSystem />;
}
