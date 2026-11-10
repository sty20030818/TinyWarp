import AppKit
import ApplicationServices
import CoreGraphics
import Foundation

NSApplication.shared.setActivationPolicy(.prohibited)

let args = CommandLine.arguments
if args.count == 2 && args[1] == "permissions" {
  print(AXIsProcessTrusted() ? "{\"accessibility\":true}" : "{\"accessibility\":false}")
  exit(0)
}
guard args.count >= 3 else { exit(2) }
let path = URL(fileURLWithPath: args[1]).standardizedFileURL
guard let bundle = Bundle(url: path), let id = bundle.bundleIdentifier,
  id.hasPrefix("dev.warp.TinyWarpL3A")
else { exit(3) }
let apps = NSRunningApplication.runningApplications(withBundleIdentifier: id)
  .filter {
    $0.bundleURL?.resolvingSymlinksInPath().standardizedFileURL.path
      == path.resolvingSymlinksInPath().standardizedFileURL.path
  }
guard apps.count == 1, let app = apps.first else { exit(4) }
let pid = app.processIdentifier
func emit(_ value: [String: Any]) {
  let data = try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
  print(String(decoding: data, as: UTF8.self))
}
func pressMenu(_ topTitle: String, _ itemTitle: String) {
  guard app.isActive else { exit(12) }
  func attribute(_ e: AXUIElement, _ n: String) -> CFTypeRef? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(e, n as CFString, &value) == .success else { return nil }
    return value
  }
  func children(_ e: AXUIElement) -> [AXUIElement] {
    attribute(e, kAXChildrenAttribute) as? [AXUIElement] ?? []
  }
  let element = AXUIElementCreateApplication(pid)
  guard let raw = attribute(element, kAXMenuBarAttribute) else { exit(7) }
  let bar = raw as! AXUIElement
  guard
    let top = children(bar).first(where: { attribute($0, kAXTitleAttribute) as? String == topTitle }
    ),
    let item = children(top).flatMap(children).first(where: {
      attribute($0, kAXTitleAttribute) as? String == itemTitle
    })
  else { exit(8) }
  guard (attribute(item, kAXEnabledAttribute) as? NSNumber)?.boolValue == true else {
    emit(["error": "menu_disabled", "active": app.isActive, "menu": topTitle, "item": itemTitle])
    exit(10)
  }
  guard AXUIElementPerformAction(item, kAXPressAction as CFString) == .success else { exit(9) }
}
switch args[2] {
case "info":
  let all =
    CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID)
    as? [[String: Any]] ?? []
  let windows = all.filter {
    ($0[kCGWindowOwnerPID as String] as? NSNumber)?.int32Value == pid
      && ($0[kCGWindowLayer as String] as? NSNumber)?.intValue == 0
  }
  emit([
    "pid": pid, "bundle_id": id, "active": app.isActive, "hidden": app.isHidden,
    "frontmost_bundle_id": NSWorkspace.shared.frontmostApplication?.bundleIdentifier ?? "",
    "frontmost_pid": NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,
    "controller_pid": ProcessInfo.processInfo.processIdentifier,
    "windows": windows.map {
      ["id": $0[kCGWindowNumber as String] ?? 0, "bounds": $0[kCGWindowBounds as String] ?? [:]]
    },
  ])
case "menus":
  let element = AXUIElementCreateApplication(pid)
  func attribute(_ e: AXUIElement, _ n: String) -> CFTypeRef? {
    var value: CFTypeRef?
    guard AXUIElementCopyAttributeValue(e, n as CFString, &value) == .success else { return nil }
    return value
  }
  func children(_ e: AXUIElement) -> [AXUIElement] {
    attribute(e, kAXChildrenAttribute) as? [AXUIElement] ?? []
  }
  guard let bar = attribute(element, kAXMenuBarAttribute) else { exit(7) }
  let items = children(bar as! AXUIElement)
  var report: [String: Any] = [:]
  for top in items where (attribute(top, kAXTitleAttribute) as? String) != "Apple" {
    let title = attribute(top, kAXTitleAttribute) as? String ?? ""
    report[title] = children(top).flatMap(children).map {
      attribute($0, kAXTitleAttribute) as? String ?? ""
    }
  }
  emit(report)
case "display-info":
  let display = CGMainDisplayID()
  let bounds = CGDisplayBounds(display)
  emit([
    "display_id": display, "display_x": bounds.minX, "display_y": bounds.minY,
    "display_width": bounds.width, "display_height": bounds.height,
    "pixel_width": CGDisplayPixelsWide(display), "pixel_height": CGDisplayPixelsHigh(display),
    "target_width": 992, "target_height": 684,
  ])
case "activate":
  let status = AXUIElementSetAttributeValue(
    AXUIElementCreateApplication(pid), kAXFrontmostAttribute as CFString, kCFBooleanTrue)
  emit(["ax_frontmost_status": status.rawValue])
case "hide":
  emit(["hidden": app.hide()])
case "quit":
  emit(["normal_termination_requested": app.terminate()])
case "new-tab": pressMenu("文件", "新建终端标签页")
case "focus-input": pressMenu("编辑", "聚焦终端输入区")
case "split": pressMenu("标签页", "向右拆分窗格")
case "close": pressMenu("文件", "关闭当前会话")
default: exit(2)
}
