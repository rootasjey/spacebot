import AppKit
import SwiftRs

@_cdecl("set_titlebar_style")
public func setTitlebarStyle(window: NSWindow, fullScreen: Bool) {
    window.titlebarAppearsTransparent = true
    window.styleMask.insert(.fullSizeContentView)
    if fullScreen {
        window.toolbar = nil
    }
    window.titleVisibility = fullScreen ? .visible : .hidden
}

@_cdecl("lock_app_theme")
public func lockAppTheme(themeType: Int) {
    let theme: NSAppearance?
    switch themeType {
    case 0:
        theme = NSAppearance(named: .aqua)
    case 1:
        theme = NSAppearance(named: .darkAqua)
    case -1:
        theme = nil
    default:
        theme = nil
    }
    NSApp.appearance = theme
}

@_cdecl("get_system_appearance")
public func getSystemAppearance() -> Int {
    // Reads the system-wide appearance preference, ignoring any per-app
    // NSApp.appearance override. "AppleInterfaceStyle" is "Dark" when the
    // system is in dark mode and nil (absent) for light mode.
    if UserDefaults.standard.string(forKey: "AppleInterfaceStyle") == "Dark" {
        return 1
    }
    return 0
}
