//! Production `IUIAutomation` (AccessKit) inspector, top-level window enumerator,
//! and 3-tier non-intrusive background input actuator (`M2.2`).

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use ui_qa_core::model::{BoundingRect, ControlSource, UiAction, UiElement};
use ui_qa_sdk::{send_sdk_command, SdkCommand};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
    UIA_InvokePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GetClassNameW, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CHAR,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
};

const MK_LBUTTON_FLAG: usize = 0x0001;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredWindow {
    pub hwnd: isize,
    pub pid: u32,
    pub title: String,
    pub width: u32,
    pub height: u32,
}

/// Lists all visible top-level windows with non-empty titles on the current desktop.
pub fn list_top_level_windows() -> Vec<DiscoveredWindow> {
    let mut list: Vec<DiscoveredWindow> = Vec::new();
    let ptr = &mut list as *mut Vec<DiscoveredWindow> as isize;
    // SAFETY: Passing a stack-owned mutable Vec pointer to synchronous EnumWindows callback.
    unsafe {
        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(ptr));
    }
    list
}

// SAFETY: Invoked synchronously by EnumWindows on the calling thread.
unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    if !IsWindowVisible(hwnd).as_bool() {
        return BOOL(1);
    }
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return BOOL(1);
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let read_len = GetWindowTextW(hwnd, &mut buf);
    if read_len <= 0 {
        return BOOL(1);
    }
    let title = String::from_utf16_lossy(&buf[..read_len as usize]);
    if title.trim().is_empty() {
        return BOOL(1);
    }

    let mut rect = RECT::default();
    let _ = GetWindowRect(hwnd, &mut rect);
    let width = (rect.right - rect.left).max(0) as u32;
    let height = (rect.bottom - rect.top).max(0) as u32;
    if width < 80 || height < 60 {
        return BOOL(1);
    }

    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));

    let list = &mut *(lparam.0 as *mut Vec<DiscoveredWindow>);
    list.push(DiscoveredWindow {
        hwnd: hwnd.0 as isize,
        pid,
        title,
        width,
        height,
    });
    BOOL(1)
}

/// Discovers all descendant child window HWNDs of a given parent window.
pub fn find_child_windows_for_hwnd(hwnd: HWND) -> Vec<HWND> {
    let mut list = Vec::new();
    let ptr = &mut list as *mut Vec<HWND> as isize;
    // SAFETY: Passing a stack-owned mutable Vec pointer to synchronous EnumChildWindows callback.
    unsafe {
        let _ = EnumChildWindows(hwnd, Some(enum_child_proc), LPARAM(ptr));
    }
    list
}

// SAFETY: Invoked synchronously by EnumChildWindows on the calling thread.
unsafe extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<HWND>);
    list.push(hwnd);
    BOOL(1)
}

/// Extracts the UIA / AccessKit element tree for `hwnd_raw`, normalizing coordinates to client/window relative bounds.
pub fn inspect_window_uia_elements(hwnd_raw: isize) -> Result<Vec<UiElement>, String> {
    // SAFETY: Initializing COM in MTA mode on current thread.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let hwnd = HWND(hwnd_raw as *mut _);
    let mut win_rect = RECT::default();
    // SAFETY: GetWindowRect reads window screen origin so we can convert screen BoundingRectangles to window-relative coords.
    unsafe {
        let _ = GetWindowRect(hwnd, &mut win_rect);
    }
    let origin_x = win_rect.left;
    let origin_y = win_rect.top;

    let mut out = Vec::new();
    // SAFETY: Creating in-proc CUIAutomation instance and walking descendants with ControlViewWalker.
    unsafe {
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("CoCreateInstance(CUIAutomation) failed: {e}"))?;
        let walker = uia
            .ControlViewWalker()
            .map_err(|e| format!("ControlViewWalker failed: {e}"))?;

        let mut idx = 0usize;
        if let Ok(root_el) = uia.ElementFromHandle(hwnd) {
            walk_uia_subtree(
                &walker, &root_el, 1, 20, &mut idx, 150, origin_x, origin_y, &mut out,
            );
        }

        let child_hwnds = find_child_windows_for_hwnd(hwnd);
        for child in &child_hwnds {
            if let Ok(child_el) = uia.ElementFromHandle(*child) {
                walk_uia_subtree(
                    &walker, &child_el, 1, 20, &mut idx, 150, origin_x, origin_y, &mut out,
                );
            }
        }

        // Chromium / WebView2 accessibility tree is constructed asynchronously on first query.
        // If web controls haven't hydrated yet, wait 350ms and query child HWNDs once more.
        if out.len() < 10 && !child_hwnds.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(350));
            for child in &child_hwnds {
                if let Ok(child_el) = uia.ElementFromHandle(*child) {
                    walk_uia_subtree(
                        &walker, &child_el, 1, 20, &mut idx, 150, origin_x, origin_y, &mut out,
                    );
                }
            }
        }
    }

    out.sort_by_key(|e| (e.bounds.y, e.bounds.x));
    out.dedup_by(|a, b| {
        a.bounds == b.bounds && a.name == b.name && a.control_type_id == b.control_type_id
    });

    Ok(out)
}

// SAFETY: Caller provides valid `IUIAutomationTreeWalker` and parent `IUIAutomationElement` on a COM-initialized thread.
#[allow(clippy::too_many_arguments)]
unsafe fn walk_uia_subtree(
    walker: &windows::Win32::UI::Accessibility::IUIAutomationTreeWalker,
    parent: &IUIAutomationElement,
    depth: usize,
    max_depth: usize,
    idx: &mut usize,
    max_elements: usize,
    origin_x: i32,
    origin_y: i32,
    out: &mut Vec<UiElement>,
) {
    if depth > max_depth || *idx >= max_elements {
        return;
    }
    let Ok(first_child) = walker.GetFirstChildElement(parent) else {
        return;
    };
    let mut current_opt: Option<IUIAutomationElement> = Some(first_child);
    while let Some(el) = current_opt {
        if *idx >= max_elements {
            break;
        }
        let control_type_id = el.CurrentControlType().map(|c| c.0).unwrap_or(0);
        // Skip non-client TitleBar (50037) and MenuBar (50010) subtrees entirely to avoid MSAA proxy timeouts
        if control_type_id != 50037 && control_type_id != 50010 {
            if let Some(ui_el) = convert_uia_element(&el, *idx, origin_x, origin_y) {
                // Only record non-structural controls or named interactive elements
                let is_structural_wrapper = matches!(control_type_id, 50026 | 50032 | 50033)
                    && ui_el.name.trim().is_empty()
                    && !ui_el.supports_invoke;
                if !is_structural_wrapper && ui_el.bounds.width > 0 && ui_el.bounds.height > 0 {
                    out.push(ui_el);
                    *idx += 1;
                }
            }
            if depth < max_depth {
                walk_uia_subtree(
                    walker,
                    &el,
                    depth + 1,
                    max_depth,
                    idx,
                    max_elements,
                    origin_x,
                    origin_y,
                    out,
                );
            }
        }
        current_opt = walker.GetNextSiblingElement(&el).ok();
    }
}

// SAFETY: Caller provides a valid `IUIAutomationElement` on a COM-initialized thread.
unsafe fn convert_uia_element(
    el: &IUIAutomationElement,
    idx: usize,
    origin_x: i32,
    origin_y: i32,
) -> Option<UiElement> {
    let control_type_id = el.CurrentControlType().map(|c| c.0).unwrap_or(0);
    // Skip non-client TitleBar (50037) to prevent MSAA proxy latency and focus on app controls
    if control_type_id == 50037 {
        return None;
    }

    let name = el.CurrentName().map(|b| b.to_string()).unwrap_or_default();
    let auto_id = el
        .CurrentAutomationId()
        .map(|b| b.to_string())
        .unwrap_or_default();
    let class_name = el
        .CurrentClassName()
        .map(|b| b.to_string())
        .unwrap_or_default();
    let is_enabled = el.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(true);
    let rect = el.CurrentBoundingRectangle().unwrap_or_default();

    // Only query InvokePattern on interactive control types per M0.3 finding
    let is_invokable_type = matches!(
        control_type_id,
        50000 | 50002 | 50005 | 50007 | 50011 | 50013 | 50019 | 50025 | 50031
    );
    let supports_invoke = if is_invokable_type {
        el.GetCurrentPattern(UIA_InvokePatternId)
            .map(|p| windows::core::Interface::cast::<IUIAutomationInvokePattern>(&p).is_ok())
            .unwrap_or(true)
    } else {
        false
    };

    let rel_x = (rect.left - origin_x).max(0);
    let rel_y = (rect.top - origin_y).max(0);
    let width = (rect.right - rect.left).max(0) as u32;
    let height = (rect.bottom - rect.top).max(0) as u32;

    let id = if !auto_id.is_empty() {
        auto_id
    } else {
        format!("uia-{idx:02}-{control_type_id}")
    };

    let role = if !class_name.is_empty() {
        class_name
    } else {
        control_type_label(control_type_id).to_string()
    };

    Some(UiElement {
        id,
        name,
        role,
        control_type_id,
        value: None,
        is_enabled,
        supports_invoke,
        bounds: BoundingRect::new(rel_x, rel_y, width, height),
        source: ControlSource::UiaAccessKit,
    })
}

fn control_type_label(id: i32) -> &'static str {
    match id {
        50000 => "Button",
        50002 => "CheckBox",
        50003 => "ComboBox",
        50004 => "Edit",
        50005 => "Hyperlink",
        50011 => "MenuItem",
        50013 => "RadioButton",
        50015 => "Slider",
        50019 => "TabItem",
        50020 => "Text",
        _ => "Control",
    }
}

/// Discovers the primary interactive render child HWND (e.g. `Chrome_RenderWidgetHostHWND` for WebView2/CEF)
/// or falls back to the topmost child / root window.
pub fn find_render_widget_target(hwnd: HWND) -> HWND {
    for child in find_child_windows_for_hwnd(hwnd) {
        let mut class_buf = vec![0u16; 128];
        let len = unsafe { GetClassNameW(child, &mut class_buf) };
        if len > 0 {
            let class_str = String::from_utf16_lossy(&class_buf[..len as usize]);
            if class_str == "Chrome_RenderWidgetHostHWND" {
                return child;
            }
        }
    }
    find_child_windows_for_hwnd(hwnd).first().copied().unwrap_or(hwnd)
}

/// Dispatches a `UiAction` in the background without stealing focus using:
/// 1. Optional `ui-qa-sdk` IPC bridge (if connected)
/// 2. Direct `PostMessageW` to `hwnd_raw` (`WM_MOUSEMOVE` + `WM_LBUTTONDOWN` + `WM_LBUTTONUP` / `WM_CHAR`)
pub fn dispatch_action_background(
    hwnd_raw: isize,
    action: &UiAction,
    sdk_addr: Option<SocketAddr>,
) -> Result<(), String> {
    if let Some(addr) = sdk_addr {
        let cmd = match action {
            UiAction::Click {
                element_id, x, y, ..
            } => SdkCommand::Click {
                id: Some(element_id.clone()),
                x: *x,
                y: *y,
            },
            UiAction::InputText { element_id, text } => SdkCommand::InputText {
                id: element_id.clone(),
                text: text.clone(),
            },
            UiAction::Toggle { element_id, .. } => SdkCommand::Click {
                id: Some(element_id.clone()),
                x: 0,
                y: 0,
            },
        };
        if send_sdk_command(addr, &cmd).is_ok() {
            return Ok(());
        }
    }

    let hwnd = HWND(hwnd_raw as *mut _);
    match action {
        UiAction::Click {
            element_name, x, y, ..
        } => {
            let _ = !element_name.trim().is_empty() && try_uia_invoke_by_name(hwnd, element_name);
            let target_hwnd = find_render_widget_target(hwnd);
            let mut top_rect = RECT::default();
            // SAFETY: GetWindowRect reads screen origin of top-level window.
            unsafe {
                let _ = GetWindowRect(hwnd, &mut top_rect);
            }
            let mut pt = POINT {
                x: top_rect.left + *x,
                y: top_rect.top + *y,
            };
            // SAFETY: ScreenToClient converts screen coordinate to target child window client coordinate.
            unsafe {
                let _ = ScreenToClient(target_hwnd, &mut pt);
            }
            let cx = pt.x.max(0) as usize & 0xFFFF;
            let cy = pt.y.max(0) as usize & 0xFFFF;
            let lparam = LPARAM(((cy << 16) | cx) as isize);
            // SAFETY: PostMessageW enqueues mouse events directly into the target HWND queue without moving the physical cursor.
            unsafe {
                PostMessageW(target_hwnd, WM_MOUSEMOVE, WPARAM(0), lparam)
                    .map_err(|e| format!("PostMessageW(WM_MOUSEMOVE) failed: {e}"))?;
                PostMessageW(target_hwnd, WM_LBUTTONDOWN, WPARAM(MK_LBUTTON_FLAG), lparam)
                    .map_err(|e| format!("PostMessageW(WM_LBUTTONDOWN) failed: {e}"))?;
                PostMessageW(target_hwnd, WM_LBUTTONUP, WPARAM(0), lparam)
                    .map_err(|e| format!("PostMessageW(WM_LBUTTONUP) failed: {e}"))?;
            }
            Ok(())
        }
        UiAction::InputText { text, .. } => {
            for ch in text.chars() {
                // SAFETY: Posting WM_CHAR messages for each character without changing foreground focus.
                unsafe {
                    PostMessageW(hwnd, WM_CHAR, WPARAM(ch as usize), LPARAM(1))
                        .map_err(|e| format!("PostMessageW(WM_CHAR) failed: {e}"))?;
                }
            }
            Ok(())
        }
        UiAction::Toggle { .. } => {
            let lparam = LPARAM(((60usize << 16) | 60usize) as isize);
            // SAFETY: Posting synthetic click to toggle target control.
            unsafe {
                let _ = PostMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(MK_LBUTTON_FLAG), lparam);
                let _ = PostMessageW(hwnd, WM_LBUTTONUP, WPARAM(0), lparam);
            }
            Ok(())
        }
    }
}

fn try_uia_invoke_by_name(hwnd: HWND, target_name: &str) -> bool {
    // SAFETY: Initializing COM and walking UIA tree up to depth 20 to invoke a matching element via IUIAutomationInvokePattern.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let Ok(uia) =
            CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        else {
            return false;
        };
        let Ok(walker) = uia.ControlViewWalker() else {
            return false;
        };
        if let Ok(root_el) = uia.ElementFromHandle(hwnd) {
            if invoke_matching_in_subtree(&walker, &root_el, target_name, 1, 20) {
                return true;
            }
        }
        for child in find_child_windows_for_hwnd(hwnd) {
            if let Ok(child_el) = uia.ElementFromHandle(child) {
                if invoke_matching_in_subtree(&walker, &child_el, target_name, 1, 20) {
                    return true;
                }
            }
        }
        false
    }
}

// SAFETY: Caller provides valid `IUIAutomationTreeWalker` and `IUIAutomationElement` on a COM-initialized thread.
unsafe fn invoke_matching_in_subtree(
    walker: &windows::Win32::UI::Accessibility::IUIAutomationTreeWalker,
    parent: &IUIAutomationElement,
    target_name: &str,
    depth: usize,
    max_depth: usize,
) -> bool {
    if depth > max_depth {
        return false;
    }
    let Ok(first_child) = walker.GetFirstChildElement(parent) else {
        return false;
    };
    let mut current_opt: Option<IUIAutomationElement> = Some(first_child);
    while let Some(el) = current_opt {
        let control_type_id = el.CurrentControlType().map(|c| c.0).unwrap_or(0);
        if control_type_id != 50037 && control_type_id != 50010 {
            let name = el.CurrentName().map(|b| b.to_string()).unwrap_or_default();
            if name == target_name {
                if let Ok(pat) = el.GetCurrentPattern(UIA_InvokePatternId) {
                    if let Ok(inv) =
                        windows::core::Interface::cast::<IUIAutomationInvokePattern>(&pat)
                    {
                        if inv.Invoke().is_ok() {
                            return true;
                        }
                    }
                }
            }
            if depth < max_depth
                && invoke_matching_in_subtree(walker, &el, target_name, depth + 1, max_depth)
            {
                return true;
            }
        }
        current_opt = walker.GetNextSiblingElement(&el).ok();
    }
    false
}
