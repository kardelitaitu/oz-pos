# OZ-POS Mobile Build Guide

> Build OZ-POS for Android tablets and iPads using Tauri v2 mobile.

## Prerequisites

### Android
- Android Studio (latest)
- Android SDK 34+
- JDK 17+
- Rust targets: `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`
- Tauri Android CLI: `cargo tauri android init`

### iOS (macOS only)
- Xcode 16+
- iOS SDK 18+
- Rust targets: `rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim`
- Tauri iOS CLI: `cargo tauri ios init`
- Apple Developer account (for TestFlight distribution)

## Setup

### Android
```bash
# 1. Install Android SDK (via Android Studio)
#    Set ANDROID_HOME and ANDROID_NDK_HOME environment variables

# 2. Install Rust targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

# 3. Initialize Tauri Android project
cd apps/tablet-client
cargo tauri android init

# 4. Configure signing (release builds)
#    Create a keystore and update tauri.conf.json:
#    "android": {
#      "signing": {
#        "keystore": "./oz-pos.keystore",
#        "keystorePassword": "...",
#        "keyAlias": "oz-pos",
#        "keyPassword": "..."
#      }
#    }
```

### iOS
```bash
# 1. Install Xcode from Mac App Store

# 2. Install Rust targets
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim

# 3. Initialize Tauri iOS project
cd apps/tablet-client
cargo tauri ios init

# 4. Open Xcode project and configure signing
open apps/tablet-client/gen/apple/oz-pos-tablet.xcodeproj
#    Set Team, Bundle Identifier in Signing & Capabilities
```

## Build Commands

### Development

```bash
# Android — install to connected device/emulator
cd apps/tablet-client && cargo tauri android dev

# iOS — open iOS simulator
cd apps/tablet-client && cargo tauri ios dev
```

### Release

```bash
# Android APK
cd apps/tablet-client && cargo tauri android build --release
# APK at: target/release/apk/oz-pos-tablet.apk

# Android AAB (Play Store)
# APK is generated automatically; for AAB:
# Use Android Studio: Build → Build Bundle(s) / APK(s) → Build Android App Bundle(s)

# iOS IPA (TestFlight)
cd apps/tablet-client && cargo tauri ios build --release
# IPA at: target/release/ipa/oz-pos-tablet.ipa
```

## Tablet App Architecture

The tablet client (`apps/tablet-client`) shares most code with the desktop client:
- **Shared**: All Rust crates (`oz-core`, `oz-payment`, `oz-hal`, etc.)
- **Shared**: React components and screens
- **Tablet-specific**: `ui/src/frontend/shell/tablet/` — bottom tab bar layout, touch-optimised CSS
- **Tablet-specific**: `ui/src/main.tablet.tsx` — entry point with tablet shell

Key differences from desktop:
- Bottom navigation bar instead of sidebar
- Touch targets ≥ 48px
- Larger typography (16–28px)
- Safe-area inset support for notched devices
- Scrollbar styling for touch
- No window resize constraints

## Configuration

### Android
Edit `apps/tablet-client/tauri.conf.json`:
```json
{
  "app": {
    "windows": []
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "android": {
      "minSdkVersion": 26,
      "targetSdkVersion": 34
    }
  }
}
```

### iOS
```json
{
  "bundle": {
    "ios": {
      "minSdkVersion": "16.0"
    }
  }
}
```

## Distribution

### Android
1. Build release APK
2. Sign with your keystore
3. Distribute via:
   - Sideloading (APK file)
   - Google Play Console (AAB)
   - MDM solution

### iOS
1. Build release IPA
2. Distribute via:
   - TestFlight (beta testing)
   - App Store Connect (production)
   - MDM (enterprise distribution)

## Troubleshooting

### Common Issues

**Android build fails with "Android SDK not found"**
- Set `ANDROID_HOME` to your SDK path
- Set `ANDROID_NDK_HOME` to your NDK path
- Ensure `adb` is in your PATH

**iOS build fails with "Code signing required"**
- Open Xcode project
- Select your team in Signing & Capabilities
- Use a unique bundle identifier

**Rust compilation errors**
- Ensure all Rust targets are installed
- Clean build: `cargo clean && cargo tauri android build`

**App crashes on launch**
- Check `adb logcat` for Android
- Check device console for iOS
- Ensure WAL-mode SQLite works on the device's file system

## Resources
- [Tauri Mobile Guide](https://v2.tauri.app/start/mobile/)
- [Android Developer Docs](https://developer.android.com/docs)
- [iOS Developer Docs](https://developer.apple.com/documentation/)
