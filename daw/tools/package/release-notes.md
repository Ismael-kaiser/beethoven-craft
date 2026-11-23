{INTRO}

### Downloads

| | Beethoven DAW | Its plugins for other DAWs (VST3 and CLAP) |
|---|---|---|
| Windows | `{PROGRAM}.exe` | `BeethovenDAW-Plugins-{VERSION}-windows-x86_64.zip` |
| Linux | `{PROGRAM}.AppImage` | `BeethovenDAW-Plugins-{VERSION}-linux-x86_64.zip` |
| macOS (Apple silicon) | `BeethovenDAW-{VERSION}-macos-aarch64.zip` | `BeethovenDAW-Plugins-{VERSION}-macos-aarch64.zip` |

Beethoven DAW has its own plugins built in; the plugin downloads are for using them in other DAWs. Once installed, Beethoven
Studio checks for updates when it starts (Settings ▸ General ▸ Updates) and installs them itself, under the same file
name, so shortcuts to it keep working.

- **Windows:** run the `.exe`. It isn't signed, so SmartScreen may say it doesn't recognise it: choose **More info** ▸
  **Run anyway**.
- **Linux:** make the AppImage executable (`chmod +x {PROGRAM}.AppImage`, or in its file properties) and run it.
- **macOS:** Beethoven DAW is not notarized by Apple (that needs a paid Apple Developer account the project doesn't have),
  so macOS stops it the first time. Unzip it and move **{APP}.app** to Applications. Then open it once, go to
  **System Settings ▸ Privacy & Security** and choose **Open Anyway** (on macOS 14 and earlier, right-click the app ▸
  **Open** ▸ **Open** also works). Or, in Terminal:
  `xattr -dr com.apple.quarantine "/Applications/{APP}.app"`. Updates Beethoven DAW installs itself don't ask
  again.
- **Plugins:** unzip and copy the `VST3` folder's contents to your VST3 folder and the `CLAP` folder's to your CLAP
  folder (the zip's `README.txt` lists where those are). On macOS, also run
  `xattr -dr com.apple.quarantine` on the copied plugins, or DAWs won't load them.

{CHANGES}
