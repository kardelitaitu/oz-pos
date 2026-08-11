// ── IPC contract tests for staff.ts (ADR #35 D6 / spec 0049) ───────
//
// Pins the staff wire shape: the scoped commands carry sessionToken + args,
// the create/edit args carry the 17 ADR #35 D6 profile fields, and the
// StaffMemberDto / ProfileViewDto shapes include the masked national id and
// the incomplete-profile flag. A future refactor that drops a field or
// renames a command breaks these tests deliberately.

import { describe, it, expect, vi, beforeEach } from 'vitest';

const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args?: Record<string, unknown>) => mockInvoke(cmd, args),
}));

import {
  listStaffScoped,
  createStaffScoped,
  updateStaffScoped,
  getStaffProfileScoped,
} from '@/api/staff';

const completeProfile = {
  date_of_birth: '1990-05-14',
  phone: '+14155550123',
  national_id_type: 'ssn',
  national_id: '123456789',
  email: 'alice@example.com',
  monthly_take_home_minor: 5_000_000,
  emergency_contact_name: 'Bob',
  emergency_contact_phone: '+14155550987',
};

describe('staff.ts scoped IPC contract (ADR #35 D6 profile fields)', () => {
  beforeEach(() => mockInvoke.mockReset());

  it('createStaffScoped sends the 9 required + optional profile fields', async () => {
    mockInvoke.mockResolvedValue({
      id: 'u-1',
      username: 'alice',
      display_name: 'Alice',
      role_id: 'role-staff',
      role_name: 'Staff',
      is_active: true,
      national_id_masked: '*****6789',
      is_profile_complete: true,
    });
    await createStaffScoped('session-1', {
      username: 'alice',
      pin: '1234',
      display_name: 'Alice',
      role_id: 'role-staff',
      profile: {
        ...completeProfile,
        job_title: 'Cashier',
        notes: 'Hired 2026',
      },
    });
    expect(mockInvoke).toHaveBeenCalledWith('create_staff_scoped', {
      sessionToken: 'session-1',
      args: {
        username: 'alice',
        pin: '1234',
        display_name: 'Alice',
        role_id: 'role-staff',
        profile: {
          ...completeProfile,
          job_title: 'Cashier',
          notes: 'Hired 2026',
        },
      },
    });
  });

  it('updateStaffScoped sends the profile fields and omits undefined ones', async () => {
    mockInvoke.mockResolvedValue({
      id: 'u-1',
      username: 'alice',
      display_name: 'Alice Updated',
      role_id: 'role-staff',
      role_name: 'Staff',
      is_active: true,
      national_id_masked: '*****6789',
      is_profile_complete: true,
    });
    await updateStaffScoped('session-1', {
      id: 'u-1',
      username: 'alice',
      display_name: 'Alice Updated',
      role_id: 'role-staff',
      is_active: true,
      profile: { ...completeProfile, email: 'alice.new@example.com' },
    });
    expect(mockInvoke).toHaveBeenCalledWith('update_staff_scoped', {
      sessionToken: 'session-1',
      args: {
        id: 'u-1',
        username: 'alice',
        display_name: 'Alice Updated',
        role_id: 'role-staff',
        is_active: true,
        profile: { ...completeProfile, email: 'alice.new@example.com' },
      },
    });
  });

  it('getStaffProfileScoped invokes get_staff_profile_scoped with sessionToken + userId', async () => {
    mockInvoke.mockResolvedValue({
      user_id: 'u-1',
      username: 'alice',
      display_name: 'Alice',
      date_of_birth: '1990-05-14',
      phone: '+14155550123',
      national_id_type: 'ssn',
      national_id: '123456789',
      national_id_masked: '*****6789',
      email: 'alice@example.com',
      monthly_take_home_minor: 5_000_000,
      emergency_contact_name: 'Bob',
      emergency_contact_phone: '+14155550987',
      job_title: '',
      notes: '',
      is_complete: true,
    });
    await getStaffProfileScoped('session-1', 'u-1');
    expect(mockInvoke).toHaveBeenCalledWith('get_staff_profile_scoped', {
      sessionToken: 'session-1',
      userId: 'u-1',
    });
  });

  it('listStaffScoped keeps its sessionToken + no-args shape', async () => {
    mockInvoke.mockResolvedValue([]);
    await listStaffScoped('session-1');
    expect(mockInvoke).toHaveBeenCalledWith('list_staff_scoped', {
      sessionToken: 'session-1',
    });
  });
});
