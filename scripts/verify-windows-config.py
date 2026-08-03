#!/usr/bin/env python3
r"""
scripts/verify-windows-config.py — Gate Windows release config drift.

WHY
====

The zero-popup Windows install goal (audit/28, RELEASE-06 follow-up)
rests on two properties that can silently regress:

  1. **NSIS installMode** — a `perMachine` installer requires elevation
     and reintroduces a UAC prompt at install time. The Tauri config
     must keep `bundle.windows.nsis.installMode` at `currentUser`.
  2. **asInvoker application manifest** — every shipped Windows exe must
     embed a loadable manifest (`<requestedExecutionLevel
     level="asInvoker"/>`) as the NUMERIC resource type 24
     (RT_MANIFEST). A manifest embedded as a *named* type (the string
     "RT_MANIFEST") is ignored by the Windows loader (mt.exe cannot find
     it), which previously triggered UAC installer-detection heuristics
     on every run (the updater-compat harness bug).

This script enforces both, statically (tauri.conf.json + source
app.manifest files) and — with `--exe` — against actually-built
binaries (PE resource walk). The `--exe` mode is wired into the release
workflow's Windows job so a future build-system or config change that
drops the manifest fails the release before upload.

USAGE
=====

    python3 scripts/verify-windows-config.py                      # static config + source manifests
    python3 scripts/verify-windows-config.py --exe a.exe b.exe    # PE-scan built binaries
    python3 scripts/verify-windows-config.py --verbose            # list every checked file
    python3 scripts/verify-windows-config.py --report-only        # always exit 0

EXIT CODES
==========

  * 0  all assertions hold.
  * 1  at least one violation (unless --report-only).
  * 2  a runtime error occurred (missing config/manifest/exe file).
"""

import argparse
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Every Tauri app config under apps/ — a future app that adds a Windows
# NSIS target is picked up automatically. Apps with no `bundle.windows.nsis`
# block (e.g. the tablet, Android/iOS-only) are skipped, not failed.
TAURI_CONFIGS = sorted((ROOT / "apps").glob("*/tauri.conf.json"))

# Source-level app.manifest files that must carry asInvoker. Each one is
# embedded into a shipped Windows exe (cloud-server + oz CLI via
# embed-resource build.rs, license-server via the committed Go .syso, and
# the updater-compat harness used by the release validation).
SOURCE_MANIFESTS = [
    ROOT / "apps" / "cloud-server" / "app.manifest",
    ROOT / "crates" / "oz-cli" / "app.manifest",
    ROOT / "apps" / "license-server" / "app.manifest",
    ROOT / "scripts" / "updater-compat-check" / "app.manifest",
]

REQUIRED_NSIS_INSTALL_MODE = "currentUser"
RT_MANIFEST = 24  # numeric resource type for the application manifest

DESCRIPTION = (
    "Verify Tauri NSIS installMode stays currentUser and every shipped "
    "Windows exe embeds a loadable asInvoker manifest (numeric RT_MANIFEST "
    "type 24). Prevents silent UAC-prompt regressions."
)


# ── Static config + source manifest checks ─────────────────────────────

def check_tauri_configs(verbose: bool) -> list[str]:
    """Fail if any tauri.conf.json sets NSIS installMode to perMachine."""
    errors: list[str] = []
    for path in TAURI_CONFIGS:
        if not path.is_file():
            errors.append(f"{rel(path)}: missing tauri.conf.json")
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        label = rel(path)
        nsis = (data.get("bundle") or {}).get("windows", {}).get("nsis")
        if not nsis:
            # No NSIS target configured (e.g. the tablet app is Android/iOS
            # only) — nothing to enforce. Also: if a future config deletes
            # the nsis block entirely, Tauri's NSIS default installMode is
            # already `currentUser`, so skipping is safe (perMachine can
            # only be set explicitly).
            if verbose:
                print(f"  {label}: no bundle.windows.nsis block — skipped")
            continue
        mode = nsis.get("installMode")
        if verbose:
            print(f"  {label}: bundle.windows.nsis.installMode = {mode!r}")
        if mode != REQUIRED_NSIS_INSTALL_MODE:
            errors.append(
                f"{label}: NSIS installMode is {mode!r} — must be "
                f"'{REQUIRED_NSIS_INSTALL_MODE}'. An explicit `currentUser` "
                "is required even though Tauri defaults to it; a perMachine "
                "installer requires elevation and reintroduces the UAC prompt "
                "at install time."
            )
    return errors


def check_source_manifests(verbose: bool) -> list[str]:
    """Fail if any source app.manifest lacks an asInvoker execution level."""
    errors: list[str] = []
    for path in SOURCE_MANIFESTS:
        label = rel(path)
        if not path.is_file():
            errors.append(f"{label}: missing app.manifest")
            continue
        text = path.read_text(encoding="utf-8")
        if verbose:
            print(f"  {label}: {'asInvoker' if 'asInvoker' in text else 'NO asInvoker'}")
        if "asInvoker" not in text:
            errors.append(
                f"{label}: manifest lacks <requestedExecutionLevel level=\"asInvoker\"/>"
            )
        if "requireAdministrator" in text or "highestAvailable" in text:
            errors.append(
                f"{label}: manifest requests elevation "
                "(requireAdministrator/highestAvailable) — breaks the zero-popup goal"
            )
    return errors


# ── PE resource walk (built binaries) ─────────────────────────────────

def pe_sections(data: bytes) -> dict[str, tuple[int, int, int, int]]:
    """Map section name -> (vaddr, vsize, file_offset, raw_size)."""
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe : pe + 4] != b"PE\0\0":
        raise ValueError("not a PE file")
    nsec = struct.unpack_from("<H", data, pe + 6)[0]
    opt_size = struct.unpack_from("<H", data, pe + 20)[0]
    sect_off = pe + 24 + opt_size
    sections: dict[str, tuple[int, int, int, int]] = {}
    for i in range(nsec):
        off = sect_off + i * 40
        name = data[off : off + 8].rstrip(b"\0").decode()
        vsize, vaddr, rsize, roff = struct.unpack_from("<IIII", data, off + 8)
        sections[name] = (vaddr, vsize, roff, rsize)
    return sections


def _dir_entries(data: bytes, off: int) -> list[tuple[int, int]]:
    """Read IMAGE_RESOURCE_DIRECTORY entries at file offset `off`."""
    n_named, n_id = struct.unpack_from("<HH", data, off + 12)
    entries = []
    for i in range(n_named + n_id):
        name_off, data_off = struct.unpack_from("<II", data, off + 16 + i * 8)
        entries.append((name_off, data_off))
    return entries


def find_manifest_xml(data: bytes) -> tuple[bytes | None, list[str]]:
    """Extract the embedded application manifest XML from a PE binary.

    Returns (manifest_xml_or_None, diagnostics). Only a NUMERIC type-24
    resource counts — a named-type "RT_MANIFEST" resource is ignored by
    the Windows loader and is reported as a diagnostic.

    Resource-tree addressing follows the PE spec: IMAGE_RESOURCE_DIRECTORY
    child offsets ("OffsetToData") are relative to the START of the .rsrc
    section, while the leaf IMAGE_RESOURCE_DATA_ENTRY.OffsetToData is a
    full image-base RVA (so it maps through the section table).
    """
    diag: list[str] = []
    try:
        sections = pe_sections(data)
    except ValueError as e:
        return None, [str(e)]
    if ".rsrc" not in sections:
        return None, ["no .rsrc section — manifest NOT embedded"]
    vaddr, _vsize, roff, _rsize = sections[".rsrc"]

    def rva_to_off(rva: int) -> int:
        return roff + (rva - vaddr)

    numeric_manifest: bytes | None = None
    try:
        # Level 1: resource types. The root directory sits at .rsrc start.
        for name_off, data_off in _dir_entries(data, roff):
            if name_off & 0x80000000:
                # Named type — e.g. the string "RT_MANIFEST". The Windows
                # loader never reads this for the app manifest. The name
                # string offset is also .rsrc-relative.
                soff = roff + (name_off & 0x7FFFFFFF)
                ln = struct.unpack_from("<H", data, soff)[0]
                nm = data[soff + 2 : soff + 2 + ln * 2].decode("utf-16-le", "replace")
                diag.append(f"named resource type present: {nm!r} (ignored by loader)")
                continue
            if name_off != RT_MANIFEST:
                continue
            # Level 2: name ids under type 24 (e.g. #1). Child offset is
            # relative to .rsrc start.
            lvl2 = _dir_entries(data, roff + (data_off & 0x7FFFFFFF))
            for _l2_name, l2_data in lvl2:
                # Level 3: language ids. Same .rsrc-relative addressing.
                lvl3 = _dir_entries(data, roff + (l2_data & 0x7FFFFFFF))
                for _l3_name, l3_data in lvl3:
                    # Leaf: points to IMAGE_RESOURCE_DATA_ENTRY (rsrc-rel).
                    leaf = roff + (l3_data & 0x7FFFFFFF)
                    data_rva, size = struct.unpack_from("<II", data, leaf)
                    if size and data_rva:
                        start = rva_to_off(data_rva)
                        numeric_manifest = data[start : start + size]
                        break
                if numeric_manifest:
                    break
            if not numeric_manifest:
                diag.append("numeric type-24 resource present but unreadable")
    except (struct.error, ValueError) as e:
        return None, [f"resource tree parse error: {e}"]
    return numeric_manifest, diag


def rel(path: Path) -> str:
    """Best-effort repo-relative label; falls back to the raw path."""
    try:
        return str(path.resolve().relative_to(ROOT.resolve()))
    except ValueError:
        return str(path)


def check_exe(path: Path, verbose: bool) -> list[str]:
    """Assert a built exe embeds a loadable asInvoker manifest."""
    label = rel(path)
    data = path.read_bytes()
    xml, diag = find_manifest_xml(data)
    if verbose:
        for d in diag:
            print(f"  {label}: {d}")
    errors: list[str] = []
    if xml is None:
        errors.append(f"{label}: no loadable application manifest — {diag[0] if diag else 'missing'}")
        return errors
    text = xml.decode("utf-8", "replace")
    if "asInvoker" not in text:
        errors.append(f"{label}: manifest embedded but does not request asInvoker")
    if "requireAdministrator" in text or "highestAvailable" in text:
        errors.append(f"{label}: manifest requests elevation (requireAdministrator/highestAvailable)")
    if verbose:
        print(f"  {label}: asInvoker manifest present (numeric RT_MANIFEST type 24)")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=DESCRIPTION)
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="List every checked file and its status.",
    )
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="Always exit 0; print report and return.",
    )
    parser.add_argument(
        "--exe",
        nargs="+",
        metavar="PATH",
        help="PE-scan the given built Windows executables instead of (in addition to) static checks.",
    )
    args = parser.parse_args()

    # A cp1252 Windows console must never crash instead of failing the gate.
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except (AttributeError, ValueError):
        pass

    errors: list[str] = []

    print("verify-windows-config: NSIS installMode + asInvoker manifest gate")
    if args.verbose:
        print("  tauri.conf.json checks:")
    errors += check_tauri_configs(args.verbose)
    if args.verbose:
        print("  source app.manifest checks:")
    errors += check_source_manifests(args.verbose)

    if args.exe:
        print("  --exe PE resource checks:")
        exe_paths = [Path(p) for p in args.exe]
        missing = [p for p in exe_paths if not p.is_file()]
        for p in missing:
            errors.append(f"{p}: exe file not found")
        for p in (p for p in exe_paths if p.is_file()):
            try:
                errors += check_exe(p, args.verbose)
            except (OSError, struct.error, ValueError) as e:
                errors.append(f"{p}: could not parse exe — {e}")

    print(f"verify-windows-config: {len(errors)} violation(s).")
    for e in errors:
        print(f"  ✗ {e}")

    return 0 if (args.report_only or not errors) else 1


if __name__ == "__main__":
    sys.exit(main())
