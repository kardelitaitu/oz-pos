import { ThemeProvider } from '@/components/ThemeProvider';
import DesignSystem from '@/features/design/DesignSystem';
import '@/features/design/DesignSystem.css';

export default function App() {
  return (
    <ThemeProvider>
      <DesignSystem />
    </ThemeProvider>
  );
}
