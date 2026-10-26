//! M0.3 Spike — AccessKit / `IUIAutomation` tree inspection and non-intrusive background input
//! dispatch (`IUIAutomationInvokePattern` + `PostMessageW` `WM_MOUSEMOVE`/`WM_LBUTTONDOWN`/`WM_LBUTTONUP`/`WM_CHAR`).

use serde::{Deserialize, Serialize};
use std::time::Instant;
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationInvokePattern,
    UIA_InvokePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, PostMessageW, WM_CHAR, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MOUSEMOVE,
};

const MK_LBUTTON_FLAG: usize = 0x0001;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiaElementSummary {
    pub name: String,
    pub class_name: String,
    pub control_type_id: i32,
    pub is_enabled: bool,
    pub supports_invoke_pattern: bool,
    pub bounding_rect: [i32; 4], // [x, y, width, height]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputProbeReport {
    pub hwnd: isize,
    pub uia_walk_latency_ms: f64,
    pub discovered_elements: Vec<UiaElementSummary>,
    pub uia_invoke_succeeded: bool,
    pub post_message_click_sent: bool,
    pub post_message_char_sent: bool,
    pub cursor_position_unchanged: bool,
    pub foreground_window_unchanged: bool,
}

/// Walks the target window's UIA subtree (root + direct children) and optionally invokes a child button.
pub fn inspect_and_test_background_input(
    hwnd_raw: isize,
    button_hwnd_opt: Option<isize>,
    click_client_xy: (u16, u16),
) -> anyhow::Result<InputProbeReport> {
    // SAFETY: Initializing COM in MTA mode on this worker/CLI thread; S_OK or S_FALSE or RPC_E_CHANGED_MODE are safe.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }

    let mut cursor_before = POINT::default();
    // SAFETY: GetCursorPos and GetForegroundWindow read current desktop cursor and active HWND.
    let fg_before = unsafe {
        let _ = GetCursorPos(&mut cursor_before);
        GetForegroundWindow().0 as isize
    };

    // SAFETY: Creating in-proc CUIAutomation COM object once before timing the tree walk.
    let uia: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)? };

    let start_uia = Instant::now();
    let mut elements = Vec::new();
    let mut invoke_ok = false;

    // SAFETY: Querying ElementFromHandle and walking children with ControlViewWalker on valid HWND.
    let start_uia_result = unsafe {
        let root_el = uia.ElementFromHandle(HWND(hwnd_raw as *mut _))?;
        elements.push(summarize_element(&root_el));

        let walker = uia.ControlViewWalker()?;
        let mut button_to_invoke: Option<IUIAutomationElement> = None;
        if let Ok(first_child) = walker.GetFirstChildElement(&root_el) {
            let mut current_opt: Option<IUIAutomationElement> = Some(first_child);
            while let Some(el) = current_opt {
                let summary = summarize_element(&el);
                if summary.supports_invoke_pattern && button_to_invoke.is_none() {
                    button_to_invoke = Some(el.clone());
                }
                elements.push(summary);
                current_opt = walker.GetNextSiblingElement(&el).ok();
            }
        } else if let Some(btn_raw) = button_hwnd_opt {
            if let Ok(btn_el) = uia.ElementFromHandle(HWND(btn_raw as *mut _)) {
                let summary = summarize_element(&btn_el);
                if summary.supports_invoke_pattern {
                    button_to_invoke = Some(btn_el);
                }
                elements.push(summary);
            }
        }
        let walk_ms = (start_uia.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0;

        if let Some(btn_el) = button_to_invoke {
            if let Ok(pat_unk) = btn_el.GetCurrentPattern(UIA_InvokePatternId) {
                if let Ok(invoker) =
                    windows::core::Interface::cast::<IUIAutomationInvokePattern>(&pat_unk)
                {
                    invoke_ok = invoker.Invoke().is_ok();
                }
            }
        }
        walk_ms
    };
    let uia_walk_latency_ms = start_uia_result;

    // Tier 2: Send background PostMessageW mouse & keyboard events directly to the target HWND
    let hwnd = HWND(hwnd_raw as *mut _);
    let lparam_xy =
        LPARAM(((click_client_xy.1 as usize) << 16 | (click_client_xy.0 as usize)) as isize);

    // SAFETY: PostMessageW enqueues standard window messages into the target HWND's message queue
    // without moving the system cursor or changing the foreground window.
    let (click_sent, char_sent) = unsafe {
        let m1 = PostMessageW(hwnd, WM_MOUSEMOVE, WPARAM(0), lparam_xy).is_ok();
        let m2 = PostMessageW(hwnd, WM_LBUTTONDOWN, WPARAM(MK_LBUTTON_FLAG), lparam_xy).is_ok();
        let m3 = PostMessageW(hwnd, WM_LBUTTONUP, WPARAM(0), lparam_xy).is_ok();
        let c1 = PostMessageW(hwnd, WM_CHAR, WPARAM('A' as usize), LPARAM(1)).is_ok();
        (m1 && m2 && m3, c1)
    };

    let mut cursor_after = POINT::default();
    // SAFETY: Reading cursor position and foreground window after background actuation.
    let fg_after = unsafe {
        let _ = GetCursorPos(&mut cursor_after);
        GetForegroundWindow().0 as isize
    };

    Ok(InputProbeReport {
        hwnd: hwnd_raw,
        uia_walk_latency_ms,
        discovered_elements: elements,
        uia_invoke_succeeded: invoke_ok,
        post_message_click_sent: click_sent,
        post_message_char_sent: char_sent,
        cursor_position_unchanged: cursor_before.x == cursor_after.x
            && cursor_before.y == cursor_after.y,
        foreground_window_unchanged: fg_before == fg_after,
    })
}

// SAFETY: Caller holds a valid COM-initialized thread and valid `IUIAutomationElement`.
unsafe fn summarize_element(el: &IUIAutomationElement) -> UiaElementSummary {
    let name = el.CurrentName().map(|b| b.to_string()).unwrap_or_default();
    let class_name = el
        .CurrentClassName()
        .map(|b| b.to_string())
        .unwrap_or_default();
    let control_type_id = el.CurrentControlType().map(|c| c.0).unwrap_or(0);
    let is_enabled = el.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false);
    let rect = el.CurrentBoundingRectangle().unwrap_or_default();
    // Only query InvokePattern on interactive control types (Button=50000, CheckBox=50002,
    // Hyperlink=50005, ListItem=50007, MenuItem=50011, RadioButton=50013, TabItem=50019, SplitButton=50031, Custom=50025)
    // to avoid non-client TitleBar (50037) MSAA bridge delays.
    let is_invokable_type = matches!(
        control_type_id,
        50000 | 50002 | 50005 | 50007 | 50011 | 50013 | 50019 | 50025 | 50031
    );
    let supports_invoke = if is_invokable_type {
        el.GetCurrentPattern(UIA_InvokePatternId)
            .map(|p| windows::core::Interface::cast::<IUIAutomationInvokePattern>(&p).is_ok())
            .unwrap_or(false)
    } else {
        false
    };

    UiaElementSummary {
        name,
        class_name,
        control_type_id,
        is_enabled,
        supports_invoke_pattern: supports_invoke,
        bounding_rect: [
            rect.left,
            rect.top,
            (rect.right - rect.left).max(0),
            (rect.bottom - rect.top).max(0),
        ],
    }
}
