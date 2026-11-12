//! E-02 provider agent (Stage A).
//!
//! Places a synthetic virtual-file `IDataObject` on the Windows clipboard from an
//! STA owner thread, serves `IStream` reads from a deterministic generator, logs
//! COM calls without content, samples its own memory and exits on a stop file or
//! deadline. No networking, no encryption, no product code.

#[cfg(not(windows))]
fn main() {
    eprintln!("e02-agent only runs on Windows");
    std::process::exit(2);
}

#[cfg(windows)]
mod data_object;
#[cfg(windows)]
mod descriptor;
#[cfg(windows)]
mod log;
#[cfg(windows)]
mod memory;
#[cfg(windows)]
mod stats;
#[cfg(windows)]
mod stream;

#[cfg(windows)]
fn main() {
    if let Err(e) = app::run() {
        eprintln!("e02-agent: {e}");
        log::event("fatal", serde_json::json!({ "error": e.to_string() }));
        std::process::exit(1);
    }
}

#[cfg(windows)]
mod app {
    use crate::data_object::{Formats, Shared, VirtualFileDataObject, format_name};
    use crate::{log, memory, stats};
    use clap::{Parser, ValueEnum};
    use e02_synth::Case;
    use serde_json::json;
    use std::path::PathBuf;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WAIT_TIMEOUT, WPARAM};
    use windows::Win32::System::Threading::{CreateMutexW, OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    use windows::Win32::System::Com::IDataObject;
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, GetClipboardOwner, GetClipboardSequenceNumber, RemoveClipboardFormatListener,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Ole::{OleFlushClipboard, OleInitialize, OleIsCurrentClipboard, OleSetClipboard, OleUninitialize};
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE, KillTimer, MSG,
        PostQuitMessage, RegisterClassW, SetTimer, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE,
        WM_TIMER, WNDCLASSW,
    };
    use windows::core::w;

    #[derive(Clone, Copy, ValueEnum, Debug)]
    enum ExitMode {
        /// OleSetClipboard(NULL) if our object is still current (default).
        Clear,
        /// Exit without touching the clipboard.
        Leave,
    }

    #[derive(Parser, Debug)]
    #[command(about = "E-02 Stage A provider: synthetic virtual files on the clipboard")]
    struct Args {
        /// Case file (cases/<id>.toml).
        #[arg(long)]
        case: PathBuf,
        /// Run directory for events.jsonl, summary.json and the ready/stop files.
        #[arg(long)]
        run_dir: PathBuf,
        /// Run identifier recorded in every event.
        #[arg(long)]
        run_id: String,
        /// Maximum time to hold the clipboard before exiting.
        #[arg(long, default_value_t = 300)]
        hold_secs: u64,
        #[arg(long, value_enum, default_value_t = ExitMode::Clear)]
        exit_mode: ExitMode,
        /// Exit (releasing the clipboard only if still ours) when this harness process exits.
        #[arg(long)]
        parent_pid: Option<u32>,
    }

    struct StopCondition {
        stop_file: PathBuf,
        deadline: Instant,
        /// SYNCHRONIZE handle to the harness process, if a parent was given.
        parent: Option<HANDLE>,
    }

    // SAFETY: the handle is only waited on (never closed) while the process runs.
    unsafe impl Send for StopCondition {}
    unsafe impl Sync for StopCondition {}

    /// Named mutex that prevents two agents from fighting over the clipboard.
    const INSTANCE_MUTEX: windows::core::PCWSTR = w!("Local\\OneCopyE02Agent");

    static STOP: OnceLock<StopCondition> = OnceLock::new();

    thread_local! {
        /// Our clipboard object, visible to the window procedure on the owner STA thread.
        static CURRENT: std::cell::RefCell<Option<IDataObject>> = const { std::cell::RefCell::new(None) };
    }
    const TIMER_ID: usize = 1;

    pub fn run() -> Result<(), Box<dyn std::error::Error>> {
        let args = Args::parse();
        // Single instance: refuse to start while another E-02 agent runs in this session.
        let _instance = unsafe { CreateMutexW(None, true, INSTANCE_MUTEX)? };
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return Err("another e02-agent is already running in this session; refusing to start".into());
        }
        let case = Case::load(&args.case)?;
        std::fs::create_dir_all(&args.run_dir)?;
        log::init(&args.run_dir.join("events.jsonl"), &args.run_id, &case.case_id)?;
        log::event(
            "start",
            json!({
                "agent_version": env!("CARGO_PKG_VERSION"),
                "files": case.files.iter().map(|f| json!({ "name": f.name, "size": f.size })).collect::<Vec<_>>(),
                "seed": case.seed,
                "variants": { "threading": case.variants.threading, "advertise_size": case.variants.advertise_size,
                              "progress_ui": case.variants.progress_ui },
                "exit_mode": format!("{:?}", args.exit_mode), "hold_secs": args.hold_secs,
            }),
        );
        let parent = match args.parent_pid {
            Some(pid) => Some(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid)? }),
            None => None,
        };
        log::event("watchdog", json!({ "parent_watched": parent.is_some(), "hold_secs": args.hold_secs }));
        let _ = STOP.set(StopCondition {
            stop_file: args.run_dir.join("stop"),
            deadline: Instant::now() + Duration::from_secs(args.hold_secs),
            parent,
        });
        let sampler = memory::Sampler::start();

        unsafe { OleInitialize(None)? };
        let owner = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
        stats::with(|s| s.owner_thread_id = owner);
        log::event("apartment", json!({ "owner_thread_id": owner, "apartment": "STA (OleInitialize)", "objects_agile": false }));
        let hwnd = create_listener_window()?;
        let formats = Formats::register()?;
        log::event(
            "formats_registered",
            json!({
                "descriptor": format_name(formats.descriptor), "contents": format_name(formats.contents),
                "preferred_effect": format_name(formats.preferred_effect), "marker": format_name(formats.marker),
            }),
        );
        let shared = Shared::new(case, formats, args.run_id.clone());
        let object: IDataObject = VirtualFileDataObject::create(shared);

        set_clipboard_with_retry(&object)?;
        CURRENT.with(|c| *c.borrow_mut() = Some(object.clone()));
        log::event("set_clipboard", json!({ "sequence": unsafe { GetClipboardSequenceNumber() } }));
        std::fs::write(args.run_dir.join("ready"), b"ready")?;

        unsafe { SetTimer(Some(hwnd), TIMER_ID, 200, None) };
        pump_messages();
        unsafe {
            let _ = KillTimer(Some(hwnd), TIMER_ID);
        }

        let still_current = unsafe { OleIsCurrentClipboard(&object) }.is_ok();
        let mut cleared = false;
        if matches!(args.exit_mode, ExitMode::Clear) && still_current {
            cleared = unsafe { OleSetClipboard(None) }.is_ok();
        }
        log::event("exit", json!({ "mode": format!("{:?}", args.exit_mode), "still_current": still_current, "cleared": cleared }));
        CURRENT.with(|c| *c.borrow_mut() = None);
        drop(object);

        unsafe {
            let _ = RemoveClipboardFormatListener(hwnd);
            let _ = DestroyWindow(hwnd);
            OleUninitialize();
        }
        sampler.stop();
        let summary = stats::snapshot();
        std::fs::write(args.run_dir.join("summary.json"), serde_json::to_vec_pretty(&summary)?)?;
        // Keep OleFlushClipboard referenced only to document that it is never called here (PROC-05 is Stage B).
        let _never_called: unsafe fn() -> windows::core::Result<()> = OleFlushClipboard;
        Ok(())
    }

    fn set_clipboard_with_retry(object: &IDataObject) -> windows::core::Result<()> {
        let mut last = Ok(());
        for attempt in 0..20 {
            last = unsafe { OleSetClipboard(object) };
            if last.is_ok() {
                return Ok(());
            }
            log::event("set_clipboard_retry", json!({ "attempt": attempt, "hr": format!("{:#010x}", last.as_ref().unwrap_err().code().0) }));
            std::thread::sleep(Duration::from_millis(50));
        }
        last
    }

    fn create_listener_window() -> windows::core::Result<HWND> {
        unsafe {
            let instance = GetModuleHandleW(None)?;
            let class = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                lpszClassName: w!("OneCopyE02AgentWindow"),
                ..Default::default()
            };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("OneCopyE02AgentWindow"),
                w!("OneCopy E-02 agent"),
                WINDOW_STYLE::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(instance.into()),
                None,
            )?;
            AddClipboardFormatListener(hwnd)?;
            Ok(hwnd)
        }
    }

    fn pump_messages() {
        let mut msg = MSG::default();
        unsafe {
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CLIPBOARDUPDATE => {
                let owner_is_us = unsafe { GetClipboardOwner() }.map(|h| h == hwnd).unwrap_or(false);
                // Another process replacing the clipboard ends the paste window for this run (interference).
                let still_current = CURRENT.with(|c| c.borrow().as_ref().map(|o| unsafe { OleIsCurrentClipboard(o) }.is_ok()));
                if still_current == Some(false) {
                    let now = log::now_ms();
                    stats::with(|s| {
                        s.clipboard_lost_ms.get_or_insert(now);
                    });
                }
                log::event(
                    "clipboard_update",
                    json!({
                        "sequence": unsafe { GetClipboardSequenceNumber() }, "owner_is_listener_window": owner_is_us,
                        "our_object_current": still_current,
                    }),
                );
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == TIMER_ID => {
                if let Some(stop) = STOP.get() {
                    let by_file = stop.stop_file.exists();
                    let by_deadline = Instant::now() >= stop.deadline;
                    let by_parent = stop.parent.is_some_and(|h| unsafe { WaitForSingleObject(h, 0) } != WAIT_TIMEOUT);
                    if by_file || by_deadline || by_parent {
                        log::event("stop_requested", json!({ "stop_file": by_file, "deadline": by_deadline, "parent_exited": by_parent }));
                        unsafe { PostQuitMessage(0) };
                    }
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}
