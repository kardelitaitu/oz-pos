import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import FastPINOverlay from "@/components/FastPINOverlay";

// ── Mock staff login API ────────────────────────────────────────────

const mockStaffLogin = vi.fn();
vi.mock("@/api/staff", () => ({
  staffLogin: (...args: unknown[]) => mockStaffLogin(...args),
}));

// ── Mock AuthContext ─────────────────────────────────────────────────

const mockSwapSession = vi.fn();
const mockAuthValue = {
  session: {
    user_id: "user-current",
    display_name: "Alice",
    role_name: "cashier",
    role_id: "role-cashier",
  },
  loading: false,
  error: null,
  login: vi.fn(),
  logout: vi.fn(),
  clearError: vi.fn(),
  isManager: false,
  isOwner: false,
  swapSession: mockSwapSession,
};

vi.mock("@/contexts/AuthContext", () => ({
  useAuth: () => mockAuthValue,
  AuthProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

// ── Mock WorkspaceContext ────────────────────────────────────────────

const mockSwapSessionToken = vi.fn();
vi.mock("@/contexts/WorkspaceContext", () => ({
  useWorkspace: () => ({
    swapSessionToken: mockSwapSessionToken,
    activeWorkspace: "store-pos",
    activeInstance: null,
    sessionToken: "token-abc",
    setActiveWorkspace: vi.fn(),
    setActiveInstance: vi.fn(),
    availableWorkspaces: [],
    workspaceScreens: [],
    loading: false,
    error: null,
    retry: vi.fn(),
    lastWorkspace: null,
    switchStore: vi.fn(),
    resolvedStoreId: "default",
  }),
  useWorkspaceScope: () => ({
    storeId: "default",
    instanceId: "default-store-pos",
    typeKey: "store-pos",
  }),
  WorkspaceProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

// ── Mock @fluent/react ──────────────────────────────────────────────

vi.mock("@fluent/react", () => ({
  useLocalization: () => ({
    l10n: {
      getString: (id: string) => id,
    },
  }),
  Localized: ({
    children,
  }: {
    id: string;
    children: React.ReactNode;
    attrs?: Record<string, boolean>;
    vars?: Record<string, string>;
  }) => <>{children}</>,
}));

// ── Helpers ─────────────────────────────────────────────────────────

function renderOverlay(open = true, onClose = vi.fn()) {
  return render(<FastPINOverlay open={open} onClose={onClose} />);
}

function typeUsername(name: string) {
  fireEvent.change(screen.getByPlaceholderText("Username"), { target: { value: name } });
}

function clickDigit(digit: string) {
  fireEvent.click(screen.getByLabelText(digit));
}

function clickButton(name: string | RegExp) {
  fireEvent.click(screen.getByRole('button', { name }));
}

// ── Tests ───────────────────────────────────────────────────────────

describe("FastPINOverlay", () => {
  describe("rendering", () => {
    it("renders nothing when closed", () => {
      const { container } = renderOverlay(false);
      expect(container.innerHTML).toBe("");
    });

    it("renders the overlay when open", () => {
      renderOverlay(true);
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });

    it("shows username step by default", () => {
      renderOverlay(true);
      expect(screen.getByPlaceholderText("Username")).toBeInTheDocument();
    });

    it("shows close button", () => {
      renderOverlay(true);
      const closeBtn = screen.getByLabelText("modal-close-aria");
      expect(closeBtn).toBeInTheDocument();
    });

    it("shows cancel button in footer", () => {
      renderOverlay(true);
      expect(screen.getByText("Cancel")).toBeInTheDocument();
    });

    it("focuses the username input on open", async () => {
      renderOverlay(true);
      // Small delay for the setTimeout in the component
      await waitFor(
        () => {
          expect(screen.getByPlaceholderText("Username")).toHaveFocus();
        },
        { timeout: 100 },
      );
    });
  });

  describe("username step", () => {
    it("advances to PIN step when username is entered and submitted", async () => {
      renderOverlay(true);
      typeUsername("cashier1");
      clickButton("Next");

      await waitFor(() => {
        expect(
          screen.queryByPlaceholderText("Username"),
        ).not.toBeInTheDocument();
      });
    });

    it("does not advance with empty username", () => {
      renderOverlay(true);
      const nextBtn = screen.getByText("Next");
      expect(nextBtn).toBeDisabled();
    });

    it("closes when overlay backdrop is clicked", async () => {
      const onClose = vi.fn();
      renderOverlay(true, onClose);
      // Click the backdrop (outer overlay div, no longer role="presentation")
      const overlay = document.querySelector(".fastpin-overlay")!;
      fireEvent.click(overlay);
      // Component uses 200ms exit animation before calling onClose
      await waitFor(() => {
        expect(onClose).toHaveBeenCalled();
      });
    });

    it("closes when close button is clicked", async () => {
      const onClose = vi.fn();
      renderOverlay(true, onClose);
      fireEvent.click(screen.getByLabelText("modal-close-aria"));
      // Component uses 200ms exit animation before calling onClose
      await waitFor(() => {
        expect(onClose).toHaveBeenCalled();
      });
    });
  });

  describe("PIN step", () => {
    async function advanceToPinStep() {
      renderOverlay(true);
      typeUsername("cashier1");
      clickButton("Next");
    }

    it("shows PIN dots and keypad", async () => {
      await advanceToPinStep();
      const dots = document.querySelectorAll(".fastpin-pin-dot");
      expect(dots.length).toBe(4);
      const keys = document.querySelectorAll(".fastpin-pad-key");
      expect(keys.length).toBeGreaterThanOrEqual(10);
    });

    it("fills PIN dots as digits are entered", async () => {
      await advanceToPinStep();
      clickDigit("1");
      clickDigit("2");
      clickDigit("3");

      const filled = document.querySelectorAll(".fastpin-pin-dot--filled");
      expect(filled.length).toBe(3);
    });

    it("clears PIN when clear button is pressed", async () => {
      await advanceToPinStep();
      clickDigit("1");
      clickDigit("2");

      clickButton("Clear");

      const filled = document.querySelectorAll(".fastpin-pin-dot--filled");
      expect(filled.length).toBe(0);
    });

    it("backspace removes last digit", async () => {
      await advanceToPinStep();
      clickDigit("1");
      clickDigit("2");
      clickButton("Backspace");

      const filled = document.querySelectorAll(".fastpin-pin-dot--filled");
      expect(filled.length).toBe(1);
    });

    it("goes back to username step when Back is clicked", async () => {
      await advanceToPinStep();
      clickButton("← Back");

      await waitFor(() => {
        expect(screen.getByPlaceholderText("Username")).toBeInTheDocument();
      });
    });
  });

  describe("verification", () => {
    beforeEach(() => {
      mockStaffLogin.mockReset();
      mockSwapSession.mockReset();
      mockSwapSessionToken.mockReset();
    });

    it("calls staffLogin then swapSession then swapSessionToken on success", async () => {
      const onClose = vi.fn();
      mockStaffLogin.mockResolvedValue({
        session: {
          user_id: "user-new",
          display_name: "Bob",
          role_name: "manager",
          role_id: "role-manager",
        },
      });
      mockSwapSessionToken.mockResolvedValue(undefined);

      render(<FastPINOverlay open={true} onClose={onClose} />);

      // Enter username
      typeUsername("bob");
      clickButton("Next");

      // Enter PIN
      clickDigit("1");
      clickDigit("2");
      clickDigit("3");
      clickDigit("4");

      await waitFor(() => {
        expect(mockStaffLogin).toHaveBeenCalledWith({
          username: "bob",
          pin: "1234",
        });
      });

      await waitFor(() => {
        expect(mockSwapSession).toHaveBeenCalledWith({
          user_id: "user-new",
          display_name: "Bob",
          role_name: "manager",
          role_id: "role-manager",
        });
      });

      await waitFor(() => {
        expect(mockSwapSessionToken).toHaveBeenCalledWith(
          "user-new",
          "role-manager",
        );
      });

      await waitFor(() => {
        expect(onClose).toHaveBeenCalled();
      });
    });

    it("shows error on failed verification", async () => {
      mockStaffLogin.mockRejectedValue(new Error("Invalid PIN"));

      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      // Enter username
      typeUsername("bob");
      clickButton("Next");

      // Enter enough PIN digits to trigger auto-submit
      clickDigit("1");
      clickDigit("2");
      clickDigit("3");
      clickDigit("4");
      clickDigit("5");
      clickDigit("6");

      await waitFor(() => {
        expect(screen.getByRole("alert")).toBeInTheDocument();
        expect(screen.getByText(/Invalid PIN/)).toBeInTheDocument();
      });

      // Should not have called swap
      expect(mockSwapSession).not.toHaveBeenCalled();
    });

    it("does not auto-submit with fewer than max PIN digits", async () => {
      vi.useFakeTimers();
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      typeUsername("bob");
      clickButton("Next");

      clickDigit("1");
      clickDigit("2");

      // Advance timers by 100ms — should NOT have called staffLogin
      vi.advanceTimersByTime(100);
      expect(mockStaffLogin).not.toHaveBeenCalled();

      vi.useRealTimers();
    });
  });

  describe("reset on open", () => {
    it("resets state when reopened", async () => {
      const { rerender } = render(
        <FastPINOverlay open={true} onClose={vi.fn()} />,
      );

      // Enter username and advance to PIN
      typeUsername("bob");
      clickButton("Next");

      // Enter some digits
      clickDigit("1");
      clickDigit("2");
      const filled = document.querySelectorAll(".fastpin-pin-dot--filled");
      expect(filled.length).toBe(2);

      // Close
      rerender(<FastPINOverlay open={false} onClose={vi.fn()} />);
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

      // Reopen
      rerender(<FastPINOverlay open={true} onClose={vi.fn()} />);

      // Should be back at username step with empty state
      await waitFor(() => {
        expect(screen.getByPlaceholderText("Username")).toBeInTheDocument();
      });
    });
  });
});
