/**
 * Tests for `isEditableTarget` (KEY-03) — the shared editable-target guard.
 *
 * Used by RetailPosScreen, KdsScreen, and the app shell to suppress
 * keyboard shortcuts while the user is typing into an input, textarea,
 * select, contenteditable element, or an element with an editable ARIA role.
 * A bug here means function keys fire while cashiers type — a real
 * data-loss surface.
 */

import { describe, expect, it } from 'vitest';
import { isEditableTarget } from '@/utils/isEditableTarget';

describe('isEditableTarget (KEY-03)', () => {
  it('returns false for null', () => {
    expect(isEditableTarget(null)).toBe(false);
  });

  it('returns false for undefined', () => {
    expect(isEditableTarget(undefined as unknown as EventTarget)).toBe(false);
  });

  it('returns false for a plain div', () => {
    const div = document.createElement('div');
    expect(isEditableTarget(div)).toBe(false);
  });

  it('returns true for an INPUT element', () => {
    const input = document.createElement('input');
    expect(isEditableTarget(input)).toBe(true);
  });

  it('returns true for a TEXTAREA element', () => {
    const textarea = document.createElement('textarea');
    expect(isEditableTarget(textarea)).toBe(true);
  });

  it('returns true for a SELECT element', () => {
    const select = document.createElement('select');
    expect(isEditableTarget(select)).toBe(true);
  });

  it('returns true for a contenteditable element', () => {
    const div = document.createElement('div');
    // jsdom does not implement isContentEditable — define it to mirror a
    // real contenteditable="true" element.
    Object.defineProperty(div, 'isContentEditable', { value: true, configurable: true });
    expect(isEditableTarget(div)).toBe(true);
  });

  it('returns false for a non-contenteditable element', () => {
    const div = document.createElement('div');
    Object.defineProperty(div, 'isContentEditable', { value: false, configurable: true });
    expect(isEditableTarget(div)).toBe(false);
  });

  it('returns true for elements with an editable ARIA role', () => {
    // textbox, searchbox, combobox, spinbutton, grid, treegrid, listbox
    for (const role of ['textbox', 'searchbox', 'combobox', 'spinbutton', 'grid', 'treegrid', 'listbox']) {
      const el = document.createElement('div');
      el.setAttribute('role', role);
      expect(isEditableTarget(el), `role="${role}" should be editable`).toBe(true);
    }
  });

  it('returns false for a non-editable ARIA role', () => {
    const el = document.createElement('div');
    el.setAttribute('role', 'button');
    expect(isEditableTarget(el)).toBe(false);
  });

  it('returns false for a Text node (not an HTMLElement)', () => {
    const text = document.createTextNode('hello');
    expect(isEditableTarget(text)).toBe(false);
  });

  it('returns false for a non-element EventTarget (window)', () => {
    expect(isEditableTarget(window)).toBe(false);
  });
});