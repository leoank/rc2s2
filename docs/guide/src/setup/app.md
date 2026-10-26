# The Android app

The app runs on Android 8 (API 26) and newer, on arm64 phones. There is no store listing
or prebuilt APK yet; you build it from this repository and install it with `adb`.

## Build it

Everything the build needs (the Android SDK and NDK, Gradle, a JDK and a Rust toolchain
for Android) comes from the flake's `android` shell. It is several GB the first time.

```console
$ git clone https://github.com/leoank/rc2s2 && cd rc2s2
$ nix develop .#android
$ cd android && gradle assembleDebug
```

The build compiles the Rust client core for arm64 Android first, then the app. The APK
lands at `android/app/build/outputs/apk/debug/app-debug.apk`.

## Install it

Enable USB debugging on the phone (Settings, About phone, tap *Build number* seven times,
then Developer options, *USB debugging*), connect it, and from the same shell:

```console
$ adb install -r app/build/outputs/apk/debug/app-debug.apk
```

`-r` keeps the app's data, so upgrading the same way keeps your hub address and token.

## Connect

The phone has to reach the hub, so install the [Tailscale](https://tailscale.com/download)
app and join the hub's tailnet. Then open rc2s2 and enter:

- **Hub address**: `http://<hub's tailnet name or address>:7700`, for example
  `http://hub:7700`.
- **Client token**: the token from one of the hub's client-token lines (just the token,
  not the name).

Both stay on the phone, and the app connects again by itself at launch. To change them,
use *Change hub or token* in the home screen's menu.

## Notifications

After connecting, the app asks for permission to show notifications and registers for
push. For push to work you also need the **ntfy** app on the phone as the UnifiedPush
distributor; see [Notifications](notifications.md). Without it everything else works,
you just have to open the app to see what is waiting.
