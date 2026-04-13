use anyhow::{Result, anyhow};
use std::mem;
use windows_sys::Win32::{
    Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Shell::{NIF_ICON, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW},
        WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
            DispatchMessageW, GetMessageW, IDI_APPLICATION, LoadIconW, MSG, PostQuitMessage,
            RegisterClassW, SW_HIDE, SetTimer, ShowWindow, TranslateMessage, WM_DESTROY, WM_TIMER,
            WNDCLASSW, WS_OVERLAPPEDWINDOW,
        },
    },
};

use crate::common::consts::{TRAY_POLL_INTERVAL_MS, TRAY_TIMER_ID};
use crate::common::strings::SERVICE_DISPLAY_NAME;
use crate::service_manager;

fn wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER if wparam == TRAY_TIMER_ID => {
            if !service_manager::service_is_running().unwrap_or(false) {
                unsafe { DestroyWindow(hwnd) };
            }
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub fn run() -> Result<()> {
    unsafe {
        let class_name = wide_string("BarvazDNSTray");
        let null_instance: HINSTANCE = std::ptr::null_mut();

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: null_instance,
            hIcon: LoadIconW(null_instance, IDI_APPLICATION),
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
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
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
        nid.uFlags = NIF_ICON | NIF_TIP;
        nid.hIcon = icon;

        let tip = wide_string(&format!("{SERVICE_DISPLAY_NAME} is running"));
        let len = tip.len().min(nid.szTip.len());
        nid.szTip[..len].copy_from_slice(&tip[..len]);

        Shell_NotifyIconW(NIM_ADD, &nid);

        // Set timer to poll service status
        SetTimer(hwnd, TRAY_TIMER_ID, TRAY_POLL_INTERVAL_MS, None);

        // Message loop
        let mut msg: MSG = mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        // Cleanup
        Shell_NotifyIconW(NIM_DELETE, &nid);
    }

    Ok(())
}
