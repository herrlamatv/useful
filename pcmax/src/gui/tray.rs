// Tray icon with its own thread + hidden window (Shell_NotifyIcon needs a message loop).
// Left click: open. Right click: menu (open / boost / quit). Balloons for boost + hotkeys.
// A second start of pcmax.exe sets the "show" event -> this thread shows the window.

use super::theme::icon_rgba;
use eframe::egui;
use pcmax::engine::{self, Cmd};
use pcmax::win::wide;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WAIT_OBJECT_0, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIIF_NOSOUND, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW,
    GetCursorPos, GetSystemMetrics, MsgWaitForMultipleObjects, PeekMessageW, PostMessageW, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, ShowWindow, TrackPopupMenu, TranslateMessage, HICON, ICONINFO,
    MF_CHECKED, MF_SEPARATOR, MF_STRING, MSG, PM_REMOVE, QS_ALLINPUT, SM_CXSMICON, SW_HIDE, SW_RESTORE, SW_SHOW,
    TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
    WS_OVERLAPPED,
};

const WM_TRAY: u32 = WM_APP + 1;
const WM_BALLOON: u32 = WM_APP + 2;
const WM_REMOVE: u32 = WM_APP + 3;
const ID_OPEN: usize = 1;
const ID_BOOST: usize = 2;
const ID_QUIT: usize = 3;

struct Ctx {
    egui: egui::Context,
    engine: Mutex<Sender<Cmd>>,
    boost: Mutex<Arc<AtomicBool>>,
    stopped: Mutex<Arc<AtomicBool>>,
    german: bool,
    balloon: Mutex<Option<String>>,
    icon: AtomicIsize,
    hwnd: AtomicIsize,
    taskbar_created: u32,
}

static CTX: OnceLock<Ctx> = OnceLock::new();

/// the egui window of this process (title "pcmax")
pub fn main_window() -> Option<HWND> {
    pcmax::procs::windows_of(pcmax::procs::own_pid()).into_iter().find(|h| pcmax::procs::window_title(*h) == "pcmax")
}

pub fn show_main() {
    if let Some(h) = main_window() {
        unsafe {
            ShowWindow(h, SW_SHOW);
            ShowWindow(h, SW_RESTORE);
            SetForegroundWindow(h);
        }
    }
    if let Some(c) = CTX.get() {
        c.egui.request_repaint();
    }
}

pub fn hide_main() {
    if let Some(h) = main_window() {
        unsafe { ShowWindow(h, SW_HIDE) };
    }
}

/// small Windows notification next to the tray icon (from any thread)
pub fn notify(text: &str) {
    if let Some(c) = CTX.get() {
        *c.balloon.lock().unwrap() = Some(text.to_string());
        let h = c.hwnd.load(Ordering::SeqCst);
        if h != 0 {
            unsafe { PostMessageW(h as HWND, WM_BALLOON, 0, 0) };
        }
    }
}

/// remove the icon (before the process ends)
pub fn remove() {
    if let Some(c) = CTX.get() {
        let h = c.hwnd.load(Ordering::SeqCst);
        if h != 0 {
            unsafe { PostMessageW(h as HWND, WM_REMOVE, 0, 0) };
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

/// after the engine was restarted
pub fn set_engine(engine: Sender<Cmd>, boost: Arc<AtomicBool>, stopped: Arc<AtomicBool>) {
    if let Some(c) = CTX.get() {
        *c.engine.lock().unwrap() = engine;
        *c.boost.lock().unwrap() = boost;
        *c.stopped.lock().unwrap() = stopped;
    }
}

/// `stopped` is set by the engine when it has put everything back (quit waits for it)
pub fn spawn(egui: egui::Context, engine: Sender<Cmd>, boost: Arc<AtomicBool>, stopped: Arc<AtomicBool>, german: bool) {
    let taskbar_created = unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) };
    let _ = CTX.set(Ctx {
        egui,
        engine: Mutex::new(engine),
        boost: Mutex::new(boost),
        stopped: Mutex::new(stopped),
        german,
        balloon: Mutex::new(None),
        icon: AtomicIsize::new(0),
        hwnd: AtomicIsize::new(0),
        taskbar_created,
    });
    std::thread::Builder::new().name("pcmax-tray".into()).spawn(run).expect("tray thread");
}

fn make_icon() -> HICON {
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.clamp(16, 64) as usize;
    let rgba = icon_rgba(size);
    unsafe {
        let mut bi: BITMAPINFO = std::mem::zeroed();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            // negative = top-down rows
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let color = CreateDIBSection(std::ptr::null_mut(), &bi, DIB_RGB_COLORS, &mut bits, std::ptr::null_mut(), 0);
        if color.is_null() || bits.is_null() {
            return std::ptr::null_mut();
        }
        let dst = std::slice::from_raw_parts_mut(bits as *mut u8, size * size * 4);
        for (d, s) in dst.as_chunks_mut::<4>().0.iter_mut().zip(rgba.as_chunks::<4>().0) {
            // RGBA -> BGRA
            *d = [s[2], s[1], s[0], s[3]];
        }
        let zeros = vec![0u8; size * size / 8 + 8];
        let mask = CreateBitmap(size as i32, size as i32, 1, 1, zeros.as_ptr() as *const c_void);
        let ii = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: mask, hbmColor: color };
        let icon = CreateIconIndirect(&ii);
        DeleteObject(color);
        DeleteObject(mask);
        icon
    }
}

fn nid(hwnd: HWND) -> NOTIFYICONDATAW {
    let mut n: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    n.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    n.hWnd = hwnd;
    n.uID = 1;
    n
}

fn copy_into(dst: &mut [u16], s: &str) {
    let w: Vec<u16> = s.encode_utf16().take(dst.len() - 1).collect();
    dst[..w.len()].copy_from_slice(&w);
    dst[w.len()] = 0;
}

fn add_icon(hwnd: HWND) {
    let Some(c) = CTX.get() else { return };
    let mut n = nid(hwnd);
    n.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    n.uCallbackMessage = WM_TRAY;
    n.hIcon = c.icon.load(Ordering::SeqCst) as HICON;
    copy_into(&mut n.szTip, "pcmax");
    unsafe { Shell_NotifyIconW(NIM_ADD, &n) };
}

fn menu(hwnd: HWND) {
    let Some(c) = CTX.get() else { return };
    let de = c.german;
    let boost = c.boost.lock().unwrap().load(Ordering::SeqCst);
    unsafe {
        let m = CreatePopupMenu();
        AppendMenuW(m, MF_STRING, ID_OPEN, wide(if de { "pcmax öffnen" } else { "Open pcmax" }).as_ptr());
        AppendMenuW(m, MF_STRING | if boost { MF_CHECKED } else { 0 }, ID_BOOST, wide("Boost").as_ptr());
        AppendMenuW(m, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(m, MF_STRING, ID_QUIT, wide(if de { "Beenden (alles zurücksetzen)" } else { "Quit (put everything back)" }).as_ptr());
        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        // without this the menu doesn't close when clicking elsewhere
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, pt.x, pt.y, 0, hwnd, std::ptr::null()) as usize;
        DestroyMenu(m);
        PostMessageW(hwnd, WM_NULL, 0, 0);
        match cmd {
            ID_OPEN => show_main(),
            ID_BOOST => {
                let _ = c.engine.lock().unwrap().send(Cmd::Boost(!boost));
            }
            ID_QUIT => quit(),
            _ => {}
        }
    }
}

/// engine puts everything back, then the process ends
pub fn quit() {
    if let Some(c) = CTX.get() {
        let _ = c.engine.lock().unwrap().send(Cmd::Quit);
        for _ in 0..100 {
            if c.stopped.lock().unwrap().load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let h = c.hwnd.load(Ordering::SeqCst);
        if h != 0 {
            unsafe { Shell_NotifyIconW(NIM_DELETE, &nid(h as HWND)) };
        }
    }
    std::process::exit(0);
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let Some(c) = CTX.get() else { return DefWindowProcW(hwnd, msg, wp, lp) };
    match msg {
        WM_TRAY => {
            match lp as u32 {
                WM_LBUTTONUP => show_main(),
                WM_RBUTTONUP => menu(hwnd),
                _ => {}
            }
            0
        }
        WM_BALLOON => {
            if let Some(text) = c.balloon.lock().unwrap().take() {
                let mut n = nid(hwnd);
                n.uFlags = NIF_INFO;
                copy_into(&mut n.szInfoTitle, "pcmax");
                copy_into(&mut n.szInfo, &text);
                n.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND;
                Shell_NotifyIconW(NIM_MODIFY, &n);
            }
            0
        }
        WM_REMOVE => {
            Shell_NotifyIconW(NIM_DELETE, &nid(hwnd));
            0
        }
        m if m == c.taskbar_created && m != 0 => {
            // Explorer restarted: the icon is gone, put it back
            add_icon(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn run() {
    let Some(c) = CTX.get() else { return };
    unsafe {
        let inst = GetModuleHandleW(std::ptr::null());
        let class = wide("pcmax-tray");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            0,
            class.as_ptr(),
            wide("pcmax-tray").as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            inst,
            std::ptr::null(),
        );
        c.hwnd.store(hwnd as isize, Ordering::SeqCst);
        c.icon.store(make_icon() as isize, Ordering::SeqCst);
        add_icon(hwnd);

        let show = engine::event(engine::EV_SHOW);
        let handles: Vec<_> = show.iter().map(|h| h.raw()).collect();
        loop {
            let w = MsgWaitForMultipleObjects(handles.len() as u32, handles.as_ptr(), 0, u32::MAX, QS_ALLINPUT);
            if !handles.is_empty() && w == WAIT_OBJECT_0 {
                show_main();
            }
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}
