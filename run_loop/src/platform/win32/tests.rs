use super::*;
use std::{
    io::{BufRead, BufReader, Read, Write},
    os::windows::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::mpsc,
};

const TEST_MESSAGE: u32 = WM_USER + 73;

struct TestWindow(HWND);

impl TestWindow {
    fn new(class_name: &str) -> Self {
        unsafe {
            let mut class_name = to_utf16(class_name);
            let instance = GetModuleHandleW(std::ptr::null_mut());
            let class = WNDCLASSW {
                style: 0,
                lpfnWndProc: test_window_proc,
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: 0,
                hCursor: 0,
                hbrBackground: 0,
                lpszMenuName: std::ptr::null_mut(),
                lpszClassName: class_name.as_mut_ptr(),
            };
            // Another test thread may have registered this class already.
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                0,
                class_name.as_mut_ptr(),
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                0,
                instance,
                std::ptr::null(),
            );
            assert_ne!(
                hwnd,
                0,
                "CreateWindowExW: {}",
                std::io::Error::last_os_error()
            );
            Self(hwnd)
        }
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

unsafe extern "system" fn test_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == TEST_MESSAGE {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 1);
        return 0;
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

fn unique_class(label: &str) -> String {
    format!(
        "IrondashRunLoopTest_{}_{}_{}",
        std::process::id(),
        unsafe { GetCurrentThreadId() },
        label
    )
}

fn assert_message_delivered(hwnd: HWND) {
    unsafe {
        assert_ne!(PostMessageW(hwnd, TEST_MESSAGE, 0, 0), 0);
        let mut message = MSG::default();
        assert_ne!(
            PeekMessageW(
                &mut message,
                hwnd,
                TEST_MESSAGE,
                TEST_MESSAGE,
                PM_REMOVE | PM_NOYIELD
            ),
            0
        );
        DispatchMessageW(&message);
        assert_eq!(GetWindowLongPtrW(hwnd, GWLP_USERDATA), 1);
    }
}

#[test]
fn current_thread_window_and_missing_window() {
    let class_name = unique_class("own");
    assert_eq!(find_current_thread_window(&class_name), None);
    assert!(!is_current_thread_window(0));
    let own = TestWindow::new(&class_name);
    assert_eq!(find_current_thread_window(&class_name), Some(own.0));
    assert_message_delivered(own.0);
    let destroyed = own.0;
    drop(own);
    assert!(!is_current_thread_window(destroyed));
    assert_eq!(find_current_thread_window(&class_name), None);
}

#[test]
fn skips_window_on_another_thread() {
    let class_name = unique_class("thread");
    let own = TestWindow::new(&class_name);
    let (ready_send, ready_receive) = mpsc::channel();
    let (stop_send, stop_receive) = mpsc::channel::<()>();
    let foreign_class = class_name.clone();
    let thread = std::thread::spawn(move || {
        let foreign = TestWindow::new(&foreign_class);
        ready_send.send(foreign.0).unwrap();
        let _ = stop_receive.recv();
    });
    let foreign = ready_receive.recv().unwrap();
    assert!(!is_current_thread_window(foreign));
    assert_eq!(find_current_thread_window(&class_name), Some(own.0));
    assert_message_delivered(own.0);
    drop(own);
    assert_eq!(find_current_thread_window(&class_name), None);
    drop(stop_send);
    thread.join().unwrap();
}

struct ChildWindow(Child);

impl Drop for ChildWindow {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn skips_window_in_another_process() {
    let class_name = unique_class("process");
    let own = TestWindow::new(&class_name);
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "platform::platform_impl::tests::foreign_process_window",
            "--ignored",
            "--nocapture",
        ])
        .env("IRONDASH_TEST_WINDOW_CLASS", &class_name)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .unwrap();
    let mut child = ChildWindow(child);
    let mut output = BufReader::new(child.0.stdout.take().unwrap());
    let foreign = (&mut output)
        .lines()
        .map(Result::unwrap)
        .find_map(|line| {
            line.split_once("TEST_HWND=")
                .map(|(_, value)| value.parse::<HWND>().unwrap())
        })
        .expect("Child did not create a window");
    assert!(!is_current_thread_window(foreign));
    assert_eq!(find_current_thread_window(&class_name), Some(own.0));
    assert_message_delivered(own.0);
    drop(own);
    assert_eq!(find_current_thread_window(&class_name), None);
    // Recreating our window (the opposite launch order) must also work.
    let replacement = TestWindow::new(&class_name);
    assert_eq!(find_current_thread_window(&class_name), Some(replacement.0));
    assert_message_delivered(replacement.0);
    writeln!(child.0.stdin.as_mut().unwrap(), "stop").unwrap();
    let mut remaining_output = String::new();
    output.read_to_string(&mut remaining_output).unwrap();
    assert!(child.0.wait().unwrap().success(), "{remaining_output}");
}

#[test]
#[ignore = "subprocess helper for skips_window_in_another_process"]
fn foreign_process_window() {
    let class_name = std::env::var("IRONDASH_TEST_WINDOW_CLASS").expect("Run via parent test");
    let window = TestWindow::new(&class_name);
    println!("TEST_HWND={}", window.0);
    std::io::stdout().flush().unwrap();
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).unwrap();
}

#[test]
fn poll_once_retries_missing_window_on_next_call() {
    let run_loop = PlatformRunLoop::new();
    let mut session = PollSession::new();
    session.timed_out = true;

    // A null HWND filter would incorrectly dispatch this unrelated message.
    let unrelated = TestWindow::new(&unique_class("unrelated"));
    unsafe { assert_ne!(PostMessageW(unrelated.0, TEST_MESSAGE, 0, 0), 0) };

    let runner = std::rc::Rc::new(RefCell::new(None));
    let runner_clone = runner.clone();
    // The first search runs before this callback. Do not repeat the search for
    // each subsequent message; discover the new window on the next poll call.
    let _ = run_loop.schedule(Duration::ZERO, move || {
        let window = TestWindow::new("FlutterTaskRunnerWindow");
        unsafe { assert_ne!(PostMessageW(window.0, TEST_MESSAGE, 0, 0), 0) };
        *runner_clone.borrow_mut() = Some(window);
    });
    // Process the timer deterministically via the run loop's window.
    unsafe { assert_ne!(PostMessageW(run_loop.hwnd(), WM_TIMER, 1, 0), 0) };
    run_loop.poll_once(&mut session);
    let runner_hwnd = runner.borrow().as_ref().unwrap().0;
    assert_eq!(session.flutter_hwnd, None);
    unsafe {
        assert_eq!(GetWindowLongPtrW(runner_hwnd, GWLP_USERDATA), 0);
        assert_eq!(GetWindowLongPtrW(unrelated.0, GWLP_USERDATA), 0);
    }

    run_loop.poll_once(&mut session);
    assert_eq!(session.flutter_hwnd, Some(runner_hwnd));
    unsafe {
        assert_eq!(GetWindowLongPtrW(runner_hwnd, GWLP_USERDATA), 1);
        assert_eq!(GetWindowLongPtrW(unrelated.0, GWLP_USERDATA), 0);
    }
}

#[test]
fn poll_once_replaces_cached_window_owned_by_another_thread() {
    let run_loop = PlatformRunLoop::new();
    let own = TestWindow::new("FlutterTaskRunnerWindow");
    let (ready_send, ready_receive) = mpsc::channel();
    let (stop_send, stop_receive) = mpsc::channel::<()>();
    let thread = std::thread::spawn(move || {
        let foreign = TestWindow::new("FlutterTaskRunnerWindow");
        ready_send.send(foreign.0).unwrap();
        let _ = stop_receive.recv();
    });
    let foreign = ready_receive.recv().unwrap();
    let mut session = PollSession::new();
    session.timed_out = true;
    session.flutter_hwnd = Some(foreign);
    unsafe { assert_ne!(PostMessageW(own.0, TEST_MESSAGE, 0, 0), 0) };

    run_loop.poll_once(&mut session);
    assert_eq!(session.flutter_hwnd, Some(own.0));
    unsafe { assert_eq!(GetWindowLongPtrW(own.0, GWLP_USERDATA), 1) };

    drop(stop_send);
    thread.join().unwrap();
}

#[test]
fn poll_once_rejects_cached_other_class_and_finds_flutter_window() {
    let run_loop = PlatformRunLoop::new();
    let own = TestWindow::new("FlutterTaskRunnerWindow");
    // Seed the cache with the state left by handle reuse. Do not depend on
    // Windows recycling a particular numeric HWND during the test.
    for class_name in [
        unique_class("reused"),
        "FlutterTaskRunnerWindowExtra".into(),
    ] {
        let unrelated = TestWindow::new(&class_name);
        assert!(is_current_thread_window(unrelated.0));
        let mut session = PollSession::new();
        session.timed_out = true;
        session.flutter_hwnd = Some(unrelated.0);
        unsafe {
            SetWindowLongPtrW(own.0, GWLP_USERDATA, 0);
            assert_ne!(PostMessageW(own.0, TEST_MESSAGE, 0, 0), 0);
            assert_ne!(PostMessageW(unrelated.0, TEST_MESSAGE, 0, 0), 0);
        }

        run_loop.poll_once(&mut session);
        assert_eq!(session.flutter_hwnd, Some(own.0));
        unsafe {
            assert_eq!(GetWindowLongPtrW(own.0, GWLP_USERDATA), 1);
            assert_eq!(GetWindowLongPtrW(unrelated.0, GWLP_USERDATA), 0);
        }
    }
}

#[test]
fn poll_once_rejects_cached_other_class_without_flutter_window() {
    let run_loop = PlatformRunLoop::new();
    let unrelated = TestWindow::new(&unique_class("reused_without_runner"));
    assert!(is_current_thread_window(unrelated.0));
    let mut session = PollSession::new();
    session.timed_out = true;
    session.flutter_hwnd = Some(unrelated.0);
    unsafe { assert_ne!(PostMessageW(unrelated.0, TEST_MESSAGE, 0, 0), 0) };

    run_loop.poll_once(&mut session);
    assert_eq!(session.flutter_hwnd, None);
    unsafe { assert_eq!(GetWindowLongPtrW(unrelated.0, GWLP_USERDATA), 0) };

    let own = TestWindow::new("FlutterTaskRunnerWindow");
    unsafe { assert_ne!(PostMessageW(own.0, TEST_MESSAGE, 0, 0), 0) };
    run_loop.poll_once(&mut session);
    assert_eq!(session.flutter_hwnd, Some(own.0));
    unsafe {
        assert_eq!(GetWindowLongPtrW(own.0, GWLP_USERDATA), 1);
        assert_eq!(GetWindowLongPtrW(unrelated.0, GWLP_USERDATA), 0);
    }
}
