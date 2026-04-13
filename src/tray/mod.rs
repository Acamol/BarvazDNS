use anyhow::{Result, anyhow};
use std::mem;
use std::time::SystemTime;
use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{
            NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
            Shell_NotifyIconW,
        },
        WindowsAndMessaging::{
            AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, GWLP_USERDATA, GetCursorPos, GetMessageW, GetWindowLongPtrW,
            IDI_APPLICATION, LoadIconW, MF_SEPARATOR, MF_STRING, MSG, PostQuitMessage,
            RegisterClassW, RegisterWindowMessageW, SW_HIDE, SetForegroundWindow, SetTimer,
            SetWindowLongPtrW, ShowWindow, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TrackPopupMenu,
            TranslateMessage, WM_APP, WM_COMMAND, WM_DESTROY, WM_TIMER, WNDCLASSW,
            WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::common::consts::TRAY_POLL_INTERVAL_MS;
use crate::common::message::{Request, Response};
use crate::common::strings::SERVICE_DISPLAY_NAME;
use crate::service_manager;

const TRAY_TIMER_ID: usize = 1;
const WM_TRAY_ICON: u32 = WM_APP + 1;
const IDM_FORCE_UPDATE: usize = 1001;
const IDM_OPEN_CONFIG: usize = 1002;
const IDM_EXIT: usize = 1003;

/// Registered message ID for the "TaskbarCreated" broadcast.
/// Explorer sends this when the taskbar is (re)created, e.g. after logon
/// or if explorer.exe restarts. We re-add the tray icon in response.
static mut WM_TASKBAR_CREATED: u32 = 0;

fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn query_last_update() -> Option<SystemTime> {
    let rt = tokio::runtime::Runtime::new().ok()?;
    let response = rt.block_on(Request::GetStatus.send()).ok()?;
    match response {
        Response::Status(time) => time,
        _ => None,
    }
}

fn format_tooltip() -> String {
    match query_last_update() {
        Some(time) => {
            let datetime: chrono::DateTime<chrono::Local> = time.into();
            format!(
                "{SERVICE_DISPLAY_NAME} — last update: {}",
                datetime.format("%Y-%m-%d %H:%M:%S")
            )
        }
        None => format!("{SERVICE_DISPLAY_NAME} — no updates yet"),
    }
}

fn update_tooltip(nid: &mut NOTIFYICONDATAW) {
    let tip = wide_string(&format_tooltip());
    let len = tip.len().min(nid.szTip.len());
    nid.szTip = [0; 128];
    nid.szTip[..len].copy_from_slice(&tip[..len]);
    unsafe { Shell_NotifyIconW(NIM_MODIFY, nid) };
}

fn show_context_menu(hwnd: HWND) {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }

        AppendMenuW(
            menu,
            MF_STRING,
            IDM_FORCE_UPDATE,
            wide_string("Force Update").as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_OPEN_CONFIG,
            wide_string("Open Configuration Folder").as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(menu, MF_STRING, IDM_EXIT, wide_string("Exit").as_ptr());

        let mut pt: POINT = mem::zeroed();
        GetCursorPos(&mut pt);

        SetForegroundWindow(hwnd);
        TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
    }
}

fn set_tooltip_text(hwnd: HWND, text: &str) {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr != 0 {
        let nid = unsafe { &mut *(ptr as *mut NOTIFYICONDATAW) };
        let tip = wide_string(text);
        let len = tip.len().min(nid.szTip.len());
        nid.szTip = [0; 128];
        nid.szTip[..len].copy_from_slice(&tip[..len]);
        unsafe { Shell_NotifyIconW(NIM_MODIFY, nid) };
    }
}

fn handle_menu_command(hwnd: HWND, id: usize) {
    match id {
        IDM_EXIT => {
            let msg = wide_string("This will stop the BarvazDNS service.\nAre you sure?");
            let title = wide_string(SERVICE_DISPLAY_NAME);
            let result = unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                    hwnd,
                    msg.as_ptr(),
                    title.as_ptr(),
                    windows_sys::Win32::UI::WindowsAndMessaging::MB_YESNO
                        | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONQUESTION,
                )
            };
            if result == windows_sys::Win32::UI::WindowsAndMessaging::IDYES {
                let _ = service_manager::stop_service();
                unsafe { DestroyWindow(hwnd) };
            }
        }
        IDM_FORCE_UPDATE => {
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                let _ = rt.block_on(Request::ForceUpdate.send());
            }
        }
        IDM_OPEN_CONFIG => {
            if let Ok(path) = crate::common::config::Config::get_config_directory_path() {
                let path_wide = wide_string(&path.to_string_lossy());
                let verb = wide_string("open");
                unsafe {
                    windows_sys::Win32::UI::Shell::ShellExecuteW(
                        std::ptr::null_mut(),
                        verb.as_ptr(),
                        path_wide.as_ptr(),
                        std::ptr::null(),
                        std::ptr::null(),
                        windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
                    );
                }
            }
        }
        _ => {}
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAY_ICON => {
            let event = (lparam & 0xFFFF) as u32;
            if event == windows_sys::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP {
                show_context_menu(hwnd);
            }
            0
        }
        WM_COMMAND => {
            handle_menu_command(hwnd, wparam & 0xFFFF);
            0
        }
        WM_TIMER if wparam == TRAY_TIMER_ID => {
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
            if ptr == 0 {
                return 0;
            }
            let nid = unsafe { &mut *(ptr as *mut NOTIFYICONDATAW) };
            if service_manager::service_is_running().unwrap_or(false) {
                update_tooltip(nid);
            } else {
                set_tooltip_text(
                    hwnd,
                    &format!("{SERVICE_DISPLAY_NAME} \u{2014} service is not running"),
                );
            }
            // Re-add the icon in case the initial NIM_ADD failed (taskbar not ready).
            unsafe { Shell_NotifyIconW(NIM_ADD, nid) };
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => {
            // Re-add the tray icon when Explorer restarts or the taskbar is created.
            if msg == unsafe { WM_TASKBAR_CREATED } && msg != 0 {
                let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
                if ptr != 0 {
                    let nid = unsafe { &mut *(ptr as *mut NOTIFYICONDATAW) };
                    unsafe { Shell_NotifyIconW(NIM_ADD, nid) };
                }
                return 0;
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}

pub fn run() -> Result<()> {
    // Hide the console window — the tray is a GUI-only process.
    unsafe {
        let console = windows_sys::Win32::System::Console::GetConsoleWindow();
        if !console.is_null() {
            ShowWindow(console, SW_HIDE);
        }

        WM_TASKBAR_CREATED = RegisterWindowMessageW(wide_string("TaskbarCreated").as_ptr());
    }

    unsafe {
        let class_name = wide_string("BarvazDNSTray");
        let null_instance: HINSTANCE = std::ptr::null_mut();

        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: null_instance,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };

        if RegisterClassW(&wc) == 0 {
            return Err(anyhow!("Failed to register window class"));
        }

        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide_string("BarvazDNS Tray").as_ptr(),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            null_instance,
            std::ptr::null(),
        );

        if hwnd.is_null() {
            return Err(anyhow!("Failed to create window"));
        }

        ShowWindow(hwnd, SW_HIDE);

        // MAKEINTRESOURCE(1) - load icon with resource ID 1 from the exe
        #[allow(clippy::manual_dangling_ptr)]
        let hicon = LoadIconW(
            GetModuleHandleW(std::ptr::null()) as HINSTANCE,
            1 as *const u16,
        );

        let icon = if !hicon.is_null() {
            hicon
        } else {
            LoadIconW(null_instance, IDI_APPLICATION)
        };

        let mut nid: NOTIFYICONDATAW = mem::zeroed();
        nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
        nid.uCallbackMessage = WM_TRAY_ICON;
        nid.hIcon = icon;

        let initial_tip = if service_manager::service_is_running().unwrap_or(false) {
            format_tooltip()
        } else {
            format!("{SERVICE_DISPLAY_NAME} \u{2014} service is not running")
        };
        let tip = wide_string(&initial_tip);
        let len = tip.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tip[..len]);

        Shell_NotifyIconW(NIM_ADD, &nid);

        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut nid as *mut _ as isize);

        SetTimer(hwnd, TRAY_TIMER_ID, TRAY_POLL_INTERVAL_MS, None);

        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Shell_NotifyIconW(NIM_DELETE, &nid);
    }

    Ok(())
}
