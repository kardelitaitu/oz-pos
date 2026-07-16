// ── Tests for SettingsSelect custom dropdown ─────────────────────────
//
// Covers: rendering, click behavior, keyboard navigation, portal
// rendering, touchscreen compatibility, and edge cases.

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import SettingsSelect, {
  type SettingsSelectOption,
} from '@/features/settings/SettingsSelect';

// scrollIntoView is not implemented in jsdom — mock it.
Element.prototype.scrollIntoView = vi.fn();

// ── Helpers ──────────────────────────────────────────────────────────

const OPTIONS: SettingsSelectOption[] = [
  { value: 'dot', label: '1.00 (dot)' },
  { value: 'comma', label: '1,00 (comma)' },
  { value: 'none', label: '1 (none)' },
];

function renderSelect(overrides: Partial<Parameters<typeof SettingsSelect>[0]> = {}) {
  const onChange = vi.fn();
  const props = {
    id: 'test-select',
    value: 'dot',
    options: OPTIONS,
    onChange,
    ariaLabel: 'Test select',
    placeholder: 'Select an option',
    ...overrides,
  };
  const result = render(<SettingsSelect {...props} />);
  return { ...result, onChange, rerender: (p: Partial<typeof props>) => result.rerender(<SettingsSelect {...{ ...props, ...p }} />) };
}/** Simulate a mousedown on document.body (click-outside). */
function clickOutside() {
  fireEvent.mouseDown(document.body);
}

/** Fire a keyDown event wrapped in act() so React flushes state updates. */
function keyDown(el: HTMLElement, key: string) {
  act(() => { fireEvent.keyDown(el, { key }); });
}

// ── Tests ────────────────────────────────────────────────────────────

describe('SettingsSelect', () => {
  beforeEach(() => {
    // Clean up any leftover portals from previous tests.
    document.body.querySelectorAll('.ssel-dropdown').forEach((el) => el.remove());
    vi.clearAllMocks();
  });

  // ── Rendering ─────────────────────────────────────────────────

  describe('rendering', () => {
    it('renders the trigger button with the selected option label', () => {
      renderSelect();
      const trigger = screen.getByRole('button', { name: /Test select/i });
      expect(trigger).toBeInTheDocument();
      expect(trigger).toHaveTextContent('1.00 (dot)');
      expect(trigger).not.toBeDisabled();
    });

    it('shows placeholder when no option matches the value', () => {
      renderSelect({ value: 'unknown' });
      expect(screen.getByRole('button')).toHaveTextContent('Select an option');
    });

    it('shows empty string when no placeholder and no match', () => {
      // Omit placeholder entirely (cannot pass `undefined` with exactOptionalPropertyTypes).
      const onChange = vi.fn();
      render(
        <SettingsSelect
          id="test-select"
          value="unknown"
          options={OPTIONS}
          onChange={onChange}
          ariaLabel="Test select"
        />,
      );
      const trigger = screen.getByRole('button');
      // Should not show 'undefined' text — only the empty string fallback.
      expect(trigger).not.toHaveTextContent('undefined');
    });

    it('renders with id on the trigger button', () => {
      renderSelect({ id: 'my-custom-id' });
      const trigger = screen.getByRole('button');
      expect(trigger).toHaveAttribute('id', 'my-custom-id');
    });

    it('sets aria-expanded to false when closed', () => {
      renderSelect();
      expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'false');
    });

    it('sets aria-haspopup on the trigger', () => {
      renderSelect();
      expect(screen.getByRole('button')).toHaveAttribute('aria-haspopup', 'listbox');
    });

    it('renders disabled button when disabled prop is true', () => {
      renderSelect({ disabled: true });
      expect(screen.getByRole('button')).toBeDisabled();
    });

    it('chevron is present and not rotated when closed', () => {
      renderSelect();
      const chevron = document.querySelector('.ssel-chevron');
      expect(chevron).toBeInTheDocument();
      expect(chevron).not.toHaveClass('ssel-chevron--open');
    });
  });

  // ── Click behavior ────────────────────────────────────────────

  describe('click behavior', () => {
    it('opens dropdown when trigger is clicked', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      // Dropdown should be in the portal (document.body).
      const dropdown = document.querySelector('.ssel-dropdown');
      expect(dropdown).toBeInTheDocument();
      expect(dropdown).toHaveAttribute('role', 'listbox');
    });

    it('sets aria-expanded to true when open', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      expect(screen.getByRole('button')).toHaveAttribute('aria-expanded', 'true');
    });

    it('closes dropdown when trigger is clicked again', async () => {
      const user = userEvent.setup();
      renderSelect();
      const trigger = screen.getByRole('button');
      await user.click(trigger);
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      await user.click(trigger);
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });

    it('calls onChange and closes when an option is clicked', async () => {
      const user = userEvent.setup();
      const { onChange } = renderSelect();
      // Open the dropdown.
      await user.click(screen.getByRole('button'));
      // Click the "1,00 (comma)" option.
      const option = screen.getByRole('option', { name: '1,00 (comma)' });
      await user.click(option);
      expect(onChange).toHaveBeenCalledTimes(1);
      expect(onChange).toHaveBeenCalledWith('comma');
      // Dropdown should close.
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });

    it('sets aria-selected on the matching option', async () => {
      const user = userEvent.setup();
      renderSelect({ value: 'comma' });
      await user.click(screen.getByRole('button'));
      const options = screen.getAllByRole('option');
      expect(options[0]).toHaveAttribute('aria-selected', 'false');
      expect(options[1]).toHaveAttribute('aria-selected', 'true');
      expect(options[2]).toHaveAttribute('aria-selected', 'false');
    });

    it('closes dropdown when clicking outside', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      // Click outside.
      clickOutside();
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });

    it('does NOT close when clicking inside the dropdown (portal)', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      // Mousedown on the dropdown container itself (not an option).
      const dropdown = document.querySelector('.ssel-dropdown')!;
      fireEvent.mouseDown(dropdown);
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
    });

    it('does not open when disabled', async () => {
      const user = userEvent.setup();
      renderSelect({ disabled: true });
      await user.click(screen.getByRole('button'));
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });

    it('rotates chevron when open', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      const chevron = document.querySelector('.ssel-chevron');
      expect(chevron).toHaveClass('ssel-chevron--open');
    });

    it('does not call onChange when clicking outside without selection', async () => {
      const user = userEvent.setup();
      const { onChange } = renderSelect();
      await user.click(screen.getByRole('button'));
      clickOutside();
      expect(onChange).not.toHaveBeenCalled();
    });
  });

  // ── Keyboard navigation ──────────────────────────────────────

  describe('keyboard navigation', () => {
    it('opens on Enter key', () => {
      renderSelect();
      keyDown(screen.getByRole('button'), 'Enter');
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
    });

    it('opens on Space key', () => {
      renderSelect();
      keyDown(screen.getByRole('button'), ' ');
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
    });

    it('opens on ArrowDown and sets active to first option', () => {
      renderSelect();
      keyDown(screen.getByRole('button'), 'ArrowDown');
      const dropdown = document.querySelector('.ssel-dropdown');
      expect(dropdown).toBeInTheDocument();
      // First option should have the active class.
      const firstOpt = dropdown!.querySelector('.ssel-option');
      expect(firstOpt).toHaveClass('ssel-option--active');
    });

    it('opens on ArrowUp and sets active to the selected option', () => {
      renderSelect();
      keyDown(screen.getByRole('button'), 'ArrowUp');
      const dropdown = document.querySelector('.ssel-dropdown');
      expect(dropdown).toBeInTheDocument();
      // ArrowUp while closed sets active to selectedIndex (0 for 'dot').
      const opts = dropdown!.querySelectorAll('.ssel-option');
      expect(opts[0]).toHaveClass('ssel-option--active');
    });

    it('navigates forward with ArrowDown when open', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowDown'); // opens, goes to index 0
      let opts = document.querySelectorAll('.ssel-option');
      expect(opts[0]).toHaveClass('ssel-option--active');
      keyDown(trigger, 'ArrowDown'); // navigate to index 1
      opts = document.querySelectorAll('.ssel-option');
      expect(opts[1]).toHaveClass('ssel-option--active');
    });

    it('wraps around when ArrowDown past last option', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      // Press ArrowDown 4 times (open + 3 navigations = wrap around).
      for (let i = 0; i < 4; i++) {
        keyDown(trigger, 'ArrowDown');
      }
      const opts = document.querySelectorAll('.ssel-option');
      expect(opts[0]).toHaveClass('ssel-option--active');
    });

    it('navigates backward with ArrowUp when open', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowDown'); // opens, goes to selected (index 0)
      let opts = document.querySelectorAll('.ssel-option');
      expect(opts[0]).toHaveClass('ssel-option--active');
      keyDown(trigger, 'ArrowUp'); // wraps from index 0 → last (index 2)
      opts = document.querySelectorAll('.ssel-option');
      expect(opts[2]).toHaveClass('ssel-option--active');
    });

    it('wraps around when ArrowUp past first option', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowDown'); // opens, goes to index 0
      keyDown(trigger, 'ArrowUp'); // wraps to last (index 2)
      const opts = document.querySelectorAll('.ssel-option');
      expect(opts[2]).toHaveClass('ssel-option--active');
    });

    it('confirms active option with Enter when open', () => {
      const { onChange } = renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowDown'); // opens, goes to index 0
      keyDown(trigger, 'ArrowDown'); // navigate to index 1
      keyDown(trigger, 'Enter'); // confirm
      expect(onChange).toHaveBeenCalledWith('comma');
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });

    it('confirms active option with Space when open', () => {
      const { onChange } = renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowDown'); // opens, goes to selected (index 0, 'dot')
      keyDown(trigger, 'ArrowDown'); // navigate to index 1 ('comma')
      keyDown(trigger, ' '); // confirm
      expect(onChange).toHaveBeenCalledWith('comma');
    });

    it('closes on Escape and does not call onChange', () => {
      const { onChange } = renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'Enter');
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      keyDown(trigger, 'Escape');
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
      expect(onChange).not.toHaveBeenCalled();
    });

    it('jumps to first option on Home key', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowUp'); // opens, goes to last
      keyDown(trigger, 'Home');
      const opts = document.querySelectorAll('.ssel-option');
      expect(opts[0]).toHaveClass('ssel-option--active');
    });

    it('jumps to last option on End key', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowDown'); // opens, goes to first
      keyDown(trigger, 'End');
      const opts = document.querySelectorAll('.ssel-option');
      expect(opts[2]).toHaveClass('ssel-option--active');
    });

    it('does not react to keyboard when disabled', () => {
      renderSelect({ disabled: true });
      keyDown(screen.getByRole('button'), 'ArrowDown');
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });

    it('starts with selected option active on open via click', async () => {
      const user = userEvent.setup();
      renderSelect({ value: 'comma' });
      await user.click(screen.getByRole('button'));
      const opts = document.querySelectorAll('.ssel-option');
      expect(opts[1]).toHaveClass('ssel-option--selected');
      expect(opts[1]).toHaveClass('ssel-option--active');
    });

    it('starts with selected option active on open via Enter', () => {
      renderSelect({ value: 'none' });
      keyDown(screen.getByRole('button'), 'Enter');
      const opts = document.querySelectorAll('.ssel-option');
      expect(opts[2]).toHaveClass('ssel-option--active');
    });

    it('closes on Tab without confirming when open', () => {
      const { onChange } = renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'Enter'); // open
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      act(() => { fireEvent.keyDown(trigger, { key: 'Tab' }); });
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
      expect(onChange).not.toHaveBeenCalled();
    });

    it('closes on Tab when navigating options', () => {
      const { onChange } = renderSelect();
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'Enter'); // open
      keyDown(trigger, 'ArrowDown'); // navigate to index 1
      act(() => { fireEvent.keyDown(trigger, { key: 'Tab' }); });
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
      expect(onChange).not.toHaveBeenCalled();
    });
  });

  // ── Portal rendering ─────────────────────────────────────────

  describe('portal rendering', () => {
    it('renders the dropdown in document.body', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      const dropdown = document.querySelector('.ssel-dropdown');
      expect(dropdown).toBeInTheDocument();
      // Check it's a direct child of body.
      expect(dropdown!.parentElement).toBe(document.body);
    });

    it('renders options with correct labels', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      const options = screen.getAllByRole('option');
      expect(options).toHaveLength(3);
      expect(options[0]).toHaveTextContent('1.00 (dot)');
      expect(options[1]).toHaveTextContent('1,00 (comma)');
      expect(options[2]).toHaveTextContent('1 (none)');
    });

    it('sets position: fixed on the portal dropdown', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      const dropdown = document.querySelector('.ssel-dropdown') as HTMLElement;
      // Position is set via inline style from calcPosition.
      expect(dropdown.style.position).toBe('fixed');
      expect(dropdown.style.top).toBeTruthy();
      expect(dropdown.style.left).toBeTruthy();
      expect(dropdown.style.width).toBeTruthy();
    });

    it('cleanup: removes portal from body when dropdown closes', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      expect(document.body.querySelector('.ssel-dropdown')).toBeInTheDocument();
      await user.click(screen.getByRole('button'));
      expect(document.body.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });
  });

  // ── Edge cases ───────────────────────────────────────────────

  describe('edge cases', () => {
    it('handles empty options array without crashing', async () => {
      const user = userEvent.setup();
      renderSelect({ options: [] });
      await user.click(screen.getByRole('button'));
      // Dropdown should open but contain no options.
      const dropdown = document.querySelector('.ssel-dropdown');
      expect(dropdown).toBeInTheDocument();
      expect(dropdown!.querySelectorAll('.ssel-option')).toHaveLength(0);
    });

    it('handles keyboard nav with empty options without crashing', () => {
      const { onChange } = renderSelect({ options: [] });
      const trigger = screen.getByRole('button');
      keyDown(trigger, 'ArrowDown');
      // Should open with no crash.
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      // Press Enter — no active option, so onChange should NOT be called.
      keyDown(trigger, 'Enter');
      expect(onChange).not.toHaveBeenCalled();
    });

    it('handles single option', async () => {
      const user = userEvent.setup();
      const { onChange } = renderSelect({
        options: [{ value: 'only', label: 'Only option' }],
        value: 'only',
      });
      await user.click(screen.getByRole('button'));
      const option = screen.getByRole('option', { name: 'Only option' });
      await user.click(option);
      expect(onChange).toHaveBeenCalledWith('only');
    });

    it('updates displayed label when value prop changes', () => {
      const { rerender } = renderSelect({ value: 'dot' });
      expect(screen.getByRole('button')).toHaveTextContent('1.00 (dot)');
      rerender({ value: 'none' });
      expect(screen.getByRole('button')).toHaveTextContent('1 (none)');
    });

    it('refocuses trigger after selecting an option via keyboard', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      trigger.focus();
      keyDown(trigger, 'Enter'); // open
      keyDown(trigger, 'ArrowDown'); // navigate
      keyDown(trigger, 'Enter'); // confirm + close
      // Focus should return to the trigger.
      expect(document.activeElement).toBe(trigger);
    });

    it('refocuses trigger after Escape', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      trigger.focus();
      keyDown(trigger, 'Enter'); // open
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      keyDown(trigger, 'Escape'); // close
      expect(document.activeElement).toBe(trigger);
    });

    it('options have role="option" and proper aria-selected', async () => {
      const user = userEvent.setup();
      renderSelect({ value: 'dot' });
      await user.click(screen.getByRole('button'));
      const options = screen.getAllByRole('option');
      expect(options[0]).toHaveAttribute('role', 'option');
      expect(options[0]).toHaveAttribute('aria-selected', 'true');
    });

    it('does NOT refocus trigger when clicking outside', async () => {
      const user = userEvent.setup();
      renderSelect();
      const trigger = screen.getByRole('button');
      // Open the dropdown.
      await user.click(trigger);
      expect(document.activeElement).not.toBe(trigger); // focus is on an option
      // Click outside on document.body.
      clickOutside();
      // Focus should NOT return to trigger (implies it stays wherever it was).
      // We can't easily check focus on body, but we can verify trigger is not focused.
      expect(document.activeElement).not.toBe(trigger);
    });

    it('refocuses trigger after Escape (not mouse)', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      trigger.focus();
      keyDown(trigger, 'Enter'); // open
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      keyDown(trigger, 'Escape'); // close
      expect(document.activeElement).toBe(trigger);
    });

    it('refocuses trigger after confirming with Enter (not mouse)', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      trigger.focus();
      keyDown(trigger, 'Enter'); // open
      keyDown(trigger, 'ArrowDown'); // navigate
      keyDown(trigger, 'Enter'); // confirm + close
      expect(document.activeElement).toBe(trigger);
    });

    it('does NOT steal focus when closing with Tab', () => {
      renderSelect();
      const trigger = screen.getByRole('button');
      trigger.focus();
      keyDown(trigger, 'Enter'); // open
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      // Dispatch Tab on the currently focused option.
      const focused = document.activeElement!;
      act(() => { fireEvent.keyDown(focused, { key: 'Tab' }); });
      // Dropdown closed without confirming.
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });
  });

  // ── Touchscreen compatibility ───────────────────────────────

  describe('touchscreen compatibility', () => {
    it('opens on touch (simulated click)', async () => {
      const user = userEvent.setup();
      renderSelect();
      // Simulate a tap — browser synthesizes mousedown → mouseup → click.
      await user.click(screen.getByRole('button'));
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
    });

    it('selects option on touch (simulated click)', async () => {
      const user = userEvent.setup();
      const { onChange } = renderSelect();
      await user.click(screen.getByRole('button'));
      const option = screen.getByRole('option', { name: /comma/ });
      // Simulate tap on the option.
      await user.click(option);
      expect(onChange).toHaveBeenCalledWith('comma');
    });

    it('closes on outside touch (synthesized mousedown)', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
      // Simulate tapping outside — fires mousedown on document.body.
      fireEvent.mouseDown(document.body);
      expect(document.querySelector('.ssel-dropdown')).not.toBeInTheDocument();
    });

    it('does not close when tapping inside the dropdown (mousedown on portal)', async () => {
      const user = userEvent.setup();
      renderSelect();
      await user.click(screen.getByRole('button'));
      const dropdown = document.querySelector('.ssel-dropdown')!;
      // Browser synthesizes mousedown on the option element.
      const optionButton = dropdown.querySelector('.ssel-option')!;
      fireEvent.mouseDown(optionButton);
      // Dropdown should remain open (click-outside handler sees it's inside the portal).
      expect(document.querySelector('.ssel-dropdown')).toBeInTheDocument();
    });
  });
});
