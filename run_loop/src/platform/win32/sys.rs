#[allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]
pub mod windows {
    pub type DWORD = u32;
    pub type HWND = isize;
    pub type HANDLE = isize;
    pub type LPARAM = isize;
    pub type WPARAM = usize;
    pub type LRESULT = isize;
    pub type PWSTR = *mut u16;
    pub type HINSTANCE = isize;
    pub type WINDOW_EX_STYLE = u32;
    pub type WINDOW_STYLE = u32;
    pub type HMENU = isize;
    pub type WINDOW_LONG_PTR_INDEX = i32;
    pub type HCURSOR = isize;
    pub type WNDCLASS_STYLES = u32;
    pub type HICON = isize;
    pub type HBRUSH = isize;
    pub type BOOL = i32;
    pub type WNDPROC = unsafe extern "system" fn(
        param0: HWND,
        param1: u32,
        param2: WPARAM,
        param3: LPARAM,
    ) -> LRESULT;
    pub type TIMERPROC =
        unsafe extern "system" fn(param0: HWND, param1: u32, param2: usize, param3: u32);
    pub type QUEUE_STATUS_FLAGS = u32;
    pub type MSG_WAIT_FOR_MULTIPLE_OBJECTS_EX_FLAGS = u32;
    pub type PEEK_MESSAGE_REMOVE_TYPE = u32;

    pub const GWLP_USERDATA: WINDOW_LONG_PTR_INDEX = -21i32;
    pub const IDC_ARROW: PWSTR = 32512i32 as _;

    pub const WM_NCCREATE: u32 = 129u32;
    pub const WM_NCDESTROY: u32 = 130u32;
    pub const WM_TIMER: u32 = 275u32;
    pub const WM_USER: u32 = 1024u32;

    pub const HWND_MESSAGE: isize = (-3i32) as _;

    pub const QS_POSTMESSAGE: QUEUE_STATUS_FLAGS = 8u32;
    pub const QS_TIMER: QUEUE_STATUS_FLAGS = 0x10u32;

    pub const MWMO_INPUTAVAILABLE: MSG_WAIT_FOR_MULTIPLE_OBJECTS_EX_FLAGS = 4u32;

    pub const PM_REMOVE: PEEK_MESSAGE_REMOVE_TYPE = 1u32;
    pub const PM_NOYIELD: PEEK_MESSAGE_REMOVE_TYPE = 2u32;

    #[repr(C)]
    pub struct WNDCLASSW {
        pub style: WNDCLASS_STYLES,
        pub lpfnWndProc: WNDPROC,
        pub cbClsExtra: i32,
        pub cbWndExtra: i32,
        pub hInstance: HINSTANCE,
        pub hIcon: HICON,
        pub hCursor: HCURSOR,
        pub hbrBackground: HBRUSH,
        pub lpszMenuName: PWSTR,
        pub lpszClassName: PWSTR,
    }

    #[repr(C)]
    pub struct CREATESTRUCTW {
        pub lpCreateParams: *mut ::core::ffi::c_void,
        pub hInstance: HINSTANCE,
        pub hMenu: HMENU,
        pub hwndParent: HWND,
        pub cy: i32,
        pub cx: i32,
        pub y: i32,
        pub x: i32,
        pub style: i32,
        pub lpszName: PWSTR,
        pub lpszClass: PWSTR,
        pub dwExStyle: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct POINT {
        pub x: i32,
        pub y: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    pub struct MSG {
        pub hwnd: HWND,
        pub message: u32,
        pub wParam: WPARAM,
        pub lParam: LPARAM,
        pub time: u32,
        pub pt: POINT,
    }

    #[link(name = "kernel32")]
    extern "system" {
        pub fn GetModuleHandleW(lpmodulename: PWSTR) -> HINSTANCE;
    }

    #[link(name = "user32")]
    extern "system" {
        pub fn CreateWindowExW(
            dwexstyle: WINDOW_EX_STYLE,
            lpclassname: PWSTR,
            lpwindowname: PWSTR,
            dwstyle: WINDOW_STYLE,
            x: i32,
            y: i32,
            nwidth: i32,
            nheight: i32,
            hwndparent: HWND,
            hmenu: HMENU,
            hinstance: HINSTANCE,
            lpparam: *const ::core::ffi::c_void,
        ) -> HWND;
        pub fn DestroyWindow(hWnd: HWND) -> BOOL;
        pub fn DefWindowProcW(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
        pub fn GetWindowLongPtrW(hwnd: HWND, nindex: WINDOW_LONG_PTR_INDEX) -> isize;
        pub fn LoadCursorW(hinstance: HINSTANCE, lpcursorname: PWSTR) -> HCURSOR;
        pub fn RegisterClassW(lpwndclass: *const WNDCLASSW) -> u16;
        pub fn SetWindowLongPtrW(
            hwnd: HWND,
            nindex: WINDOW_LONG_PTR_INDEX,
            dwnewlong: isize,
        ) -> isize;
        pub fn UnregisterClassW(lpclassname: PWSTR, hinstance: HINSTANCE) -> BOOL;
        pub fn DispatchMessageW(lpmsg: *const MSG) -> LRESULT;
        pub fn GetMessageW(
            lpmsg: *mut MSG,
            hwnd: HWND,
            wmsgfiltermin: u32,
            wmsgfiltermax: u32,
        ) -> BOOL;
        pub fn FindWindowExW(
            hwndparent: HWND,
            hwndchildafter: HWND,
            lpszclass: PWSTR,
            lpszwindow: PWSTR,
        ) -> HWND;
        pub fn PostMessageW(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> BOOL;
        pub fn SetTimer(
            hwnd: HWND,
            nidevent: usize,
            uelapse: u32,
            lptimerfunc: ::core::option::Option<TIMERPROC>,
        ) -> usize;
        pub fn TranslateMessage(lpmsg: *const MSG) -> BOOL;
        pub fn MsgWaitForMultipleObjectsEx(
            ncount: u32,
            phandles: *const HANDLE,
            dwmilliseconds: u32,
            dwwakemask: QUEUE_STATUS_FLAGS,
            dwflags: MSG_WAIT_FOR_MULTIPLE_OBJECTS_EX_FLAGS,
        ) -> u32;
        pub fn PeekMessageW(
            lpmsg: *mut MSG,
            hwnd: HWND,
            wmsgfiltermin: u32,
            wmsgfiltermax: u32,
            wremovemsg: PEEK_MESSAGE_REMOVE_TYPE,
        ) -> BOOL;
        pub fn GetCurrentThreadId() -> DWORD;
    }
}
