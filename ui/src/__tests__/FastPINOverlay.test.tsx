import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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

// ── Tests ───────────────────────────────────────────────────────────

describe("FastPINOverlay", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

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
      const input = screen.getByPlaceholderText("Username");
      await userEvent.type(input, "cashier1");

      const nextBtn = screen.getByText("Next");
      fireEvent.click(nextBtn);

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

    it("closes when overlay backdrop is clicked", () => {
      const onClose = vi.fn();
      renderOverlay(true, onClose);
      // Click the backdrop (the outer div)
      const overlay = screen.getByRole("presentation");
      fireEvent.click(overlay);
      expect(onClose).toHaveBeenCalled();
    });

    it("closes when close button is clicked", () => {
      const onClose = vi.fn();
      renderOverlay(true, onClose);
      fireEvent.click(screen.getByLabelText("modal-close-aria"));
      expect(onClose).toHaveBeenCalled();
    });
  });

  describe("PIN step", () => {
    async function advanceToPinStep() {
      renderOverlay(true);
      const input = screen.getByPlaceholderText("Username");
      await userEvent.type(input, "cashier1");
      fireEvent.click(screen.getByText("Next"));
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
      const digitBtn = screen.getByLabelText("1");
      fireEvent.click(digitBtn);
      fireEvent.click(screen.getByLabelText("2"));
      fireEvent.click(screen.getByLabelText("3"));

      const filled = document.querySelectorAll(".fastpin-pin-dot--filled");
      expect(filled.length).toBe(3);
    });

    it("clears PIN when clear button is pressed", async () => {
      await advanceToPinStep();
      fireEvent.click(screen.getByLabelText("1"));
      fireEvent.click(screen.getByLabelText("2"));

      fireEvent.click(screen.getByLabelText("Clear"));

      const filled = document.querySelectorAll(".fastpin-pin-dot--filled");
      expect(filled.length).toBe(0);
    });

    it("backspace removes last digit", async () => {
      await advanceToPinStep();
      fireEvent.click(screen.getByLabelText("1"));
      fireEvent.click(screen.getByLabelText("2"));
      fireEvent.click(screen.getByLabelText("Backspace"));

      const filled = document.querySelectorAll(".fastpin-pin-dot--filled");
      expect(filled.length).toBe(1);
    });

    it("goes back to username step when Back is clicked", async () => {
      await advanceToPinStep();
      fireEvent.click(screen.getByText("← Back"));

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
      await userEvent.type(screen.getByPlaceholderText("Username"), "bob");
      fireEvent.click(screen.getByText("Next"));

      // Enter PIN
      fireEvent.click(screen.getByLabelText("1"));
      fireEvent.click(screen.getByLabelText("2"));
      fireEvent.click(screen.getByLabelText("3"));
      fireEvent.click(screen.getByLabelText("4"));

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
      await userEvent.type(screen.getByPlaceholderText("Username"), "bob");
      fireEvent.click(screen.getByText("Next"));

      // Enter enough PIN digits to trigger auto-submit
      fireEvent.click(screen.getByLabelText("1"));
      fireEvent.click(screen.getByLabelText("2"));
      fireEvent.click(screen.getByLabelText("3"));
      fireEvent.click(screen.getByLabelText("4"));
      fireEvent.click(screen.getByLabelText("5"));
      fireEvent.click(screen.getByLabelText("6"));

      await waitFor(() => {
        expect(screen.getByRole("alert")).toBeInTheDocument();
        expect(screen.getByText(/Invalid PIN/)).toBeInTheDocument();
      });

      // Should not have called swap
      expect(mockSwapSession).not.toHaveBeenCalled();
    });

    it("does not auto-submit with fewer than max PIN digits", async () => {
      render(<FastPINOverlay open={true} onClose={vi.fn()} />);

      await userEvent.type(screen.getByPlaceholderText("Username"), "bob");
      fireEvent.click(screen.getByText("Next"));

      fireEvent.click(screen.getByLabelText("1"));
      fireEvent.click(screen.getByLabelText("2"));

      // Wait a tick — should NOT have called staffLogin
      await new Promise((r) => setTimeout(r, 100));
      expect(mockStaffLogin).not.toHaveBeenCalled();
    });
  });

  describe("reset on open", () => {
    it("resets state when reopened", async () => {
      const { rerender } = render(
        <FastPINOverlay open={true} onClose={vi.fn()} />,
      );

      // Enter username and advance to PIN
      await userEvent.type(screen.getByPlaceholderText("Username"), "bob");
      fireEvent.click(screen.getByText("Next"));

      // Enter some digits
      fireEvent.click(screen.getByLabelText("1"));
      fireEvent.click(screen.getByLabelText("2"));
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
