//! E-02 provider agent (Stage A and Stage B1).
//!
//! Places a synthetic virtual-file `IDataObject` on the Windows clipboard from an
//! STA owner thread, serves `IStream` reads from a deterministic generator, logs
//! COM calls without content, samples its own memory and exits on a stop file or
//! deadline. Stage B1 adds the per-file fault plan, the T2/T2A threading variants, a provider-wide
//! byte budget and the `leave` / guarded `flush` exit modes. No networking, no encryption, no
//! product code.

#[cfg(not(windows))]
fn main() {
    eprintln!("e02-agent only runs on Windows");
    std::process::exit(2);
}

#[cfg(windows)]
mod control;
#[cfg(windows)]
mod data_object;
#[cfg(windows)]
mod descriptor;
#[cfg(windows)]
mod log;
#[cfg(windows)]
mod memory;
#[cfg(windows)]
mod ownership;
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
    use crate::control::{self, StopCondition};
    use crate::data_object::{self, Formats, Shared, format_name};
    use crate::{log, memory, stats};
    use clap::{Parser, ValueEnum};
    use e02_synth::{Case, FLUSH_MAX_CASE_BYTES, Threading};
    use serde_json::json;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::{CreateMutexW, OpenProcess, PROCESS_SYNCHRONIZE};
    use windows::Win32::System::Com::IDataObject;
    use windows::Win32::System::DataExchange::{
        AddClipboardFormatListener, GetClipboardSequenceNumber, RemoveClipboardFormatListener,
    };
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::System::Ole::{OleFlushClipboard, OleInitialize, OleSetClipboard, OleUninitialize};
    use crate::ownership;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE, KillTimer, MSG,
        PostQuitMessage, RegisterClassW, SetTimer, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE,
        WM_TIMER, WNDCLASSW,
    };
    use windows::core::w;

    #[derive(Clone, Copy, ValueEnum, Debug, PartialEq)]
    enum ExitMode {
        /// Release the clipboard only if our object is still current (default).
        Clear,
        /// Exit without touching the clipboard (PROC-07): observes what OLE does at uninitialize.
        Leave,
        /// OleFlushClipboard before exit (PROC-05). Refused for cases above 64 MiB; reads stay
        /// under the provider-wide byte budget.
        Flush,
    }

    #[derive(Parser, Debug)]
    #[command(about = "E-02 provider: synthetic virtual files on the clipboard")]
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

    /// Named mutex that prevents two agents from fighting over the clipboard.
    const INSTANCE_MUTEX: windows::core::PCWSTR = w!("Local\\OneCopyE02Agent");

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
                "variants": serde_json::to_value(&case.variants).unwrap_or_default(),
                "faults": case.files.iter().map(|f| serde_json::to_value(&f.faults).unwrap_or_default()).collect::<Vec<_>>(),
                "advertised_sizes": case.files.iter().map(|f| f.descriptor_size()).collect::<Vec<_>>(),
                "stat_sizes": case.files.iter().map(|f| f.stat_size).collect::<Vec<_>>(),
                "byte_budget": case.byte_budget(),
                "exit_mode": format!("{:?}", args.exit_mode), "hold_secs": args.hold_secs,
            }),
        );
        let parent = match args.parent_pid {
            Some(pid) => Some(unsafe { OpenProcess(PROCESS_SYNCHRONIZE, false, pid)? }),
            None => None,
        };
        log::event("watchdog", json!({ "parent_watched": parent.is_some(), "hold_secs": args.hold_secs }));
        if args.exit_mode == ExitMode::Flush && case.stream_bytes() > FLUSH_MAX_CASE_BYTES {
            return Err("exit mode flush is refused for cases above 64 MiB".into());
        }
        control::install(StopCondition {
            stop_file: args.run_dir.join("stop"),
            deadline: Instant::now() + Duration::from_secs(args.hold_secs),
            parent,
        });
        let sampler = memory::Sampler::start();

        unsafe { OleInitialize(None)? };
        let owner = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
        stats::with(|s| s.owner_thread_id = owner);
        let threading = case.variants.threading;
        log::event(
            "apartment",
            json!({ "owner_thread_id": owner, "apartment": "STA (OleInitialize)", "threading": threading,
                    "data_object_agile": threading == Threading::T2A, "streams_agile": threading != Threading::T1 }),
        );
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
        let object: IDataObject = data_object::create(shared);

        set_clipboard_with_retry(&object)?;
        CURRENT.with(|c| *c.borrow_mut() = Some(object.clone()));
        log::event("set_clipboard", json!({ "sequence": unsafe { GetClipboardSequenceNumber() } }));
        std::fs::write(args.run_dir.join("ready"), b"ready")?;

        // HRESULT/return-code audit: SetTimer returns 0 on failure. Without the timer nothing
        // would poll the stop conditions, so a failure releases the clipboard and exits at once.
        if unsafe { SetTimer(Some(hwnd), TIMER_ID, 200, None) } == 0 {
            log::event("fatal", json!({ "error": "SetTimer failed; releasing the clipboard and exiting" }));
        } else {
            pump_messages();
        }
        unsafe {
            let _ = KillTimer(Some(hwnd), TIMER_ID);
        }

        // Never OleSetClipboard(NULL): it empties the clipboard whoever owns it.
        let (ours, cleared, detail) = if args.exit_mode == ExitMode::Clear {
            let r = ownership::release_if_ours(&object, hwnd);
            (r.before.ours(), r.cleared, json!({
                "opened": r.opened, "ole_hresult_before": r.before.ole_hresult,
                "owner_is_our_process_before": r.before.owner_is_our_process, "ole_hresult_after": r.after_ole_hresult,
            }))
        } else {
            let o = ownership::check(&object);
            let mut detail = json!({ "ole_hresult_before": o.ole_hresult, "owner_is_our_process_before": o.owner_is_our_process });
            if args.exit_mode == ExitMode::Flush && o.ours() {
                // Guarded (case <= 64 MiB, byte budget): renders every offered format into memory.
                let before = stats::snapshot().bytes_served;
                let t0 = Instant::now();
                let hr = unsafe { OleFlushClipboard() }.err().map_or(0, |e| e.code().0);
                let after = stats::snapshot().bytes_served;
                detail["flush"] = json!({ "hr": format!("{hr:#010x}"), "ms": t0.elapsed().as_millis() as u64,
                                          "bytes_read_during_flush": after - before });
            }
            (o.ours(), false, detail)
        };
        if !ours {
            let now = log::now_ms();
            stats::with(|s| {
                s.clipboard_lost_ms.get_or_insert(now);
            });
        }
        log::event("exit", json!({ "mode": format!("{:?}", args.exit_mode), "ours": ours, "cleared": cleared, "detail": detail }));
        CURRENT.with(|c| *c.borrow_mut() = None);
        drop(object);

        let before_uninit = stats::snapshot();
        let t0 = Instant::now();
        unsafe {
            let _ = RemoveClipboardFormatListener(hwnd);
            let _ = DestroyWindow(hwnd);
            OleUninitialize();
        }
        let after_uninit = stats::snapshot();
        log::event(
            "uninitialized",
            json!({ "ms": t0.elapsed().as_millis() as u64,
                    "reads_during_uninitialize": after_uninit.read_calls - before_uninit.read_calls,
                    "bytes_during_uninitialize": after_uninit.bytes_served - before_uninit.bytes_served,
                    "get_data_during_uninitialize": after_uninit.calls.get("IDataObject::GetData").copied().unwrap_or(0)
                        - before_uninit.calls.get("IDataObject::GetData").copied().unwrap_or(0) }),
        );
        sampler.stop();
        let summary = stats::snapshot();
        std::fs::write(args.run_dir.join("summary.json"), serde_json::to_vec_pretty(&summary)?)?;
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
            // GetMessageW returns -1 on error, which BOOL::as_bool() treats as true (endless loop).
            while control::keep_pumping(GetMessageW(&mut msg, None, 0, 0)) {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CLIPBOARDUPDATE => {
                // Another process replacing the clipboard ends the paste window for this run (interference).
                let own = CURRENT.with(|c| c.borrow().as_ref().map(ownership::check));
                if own.is_some_and(|o| !o.ours()) {
                    let now = log::now_ms();
                    stats::with(|s| {
                        s.clipboard_lost_ms.get_or_insert(now);
                    });
                }
                log::event(
                    "clipboard_update",
                    json!({
                        "sequence": unsafe { GetClipboardSequenceNumber() },
                        "ole_hresult": own.map(|o| o.ole_hresult), "owner_is_our_process": own.map(|o| o.owner_is_our_process),
                        "ours": own.map(|o| o.ours()),
                    }),
                );
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == TIMER_ID => {
                let (by_file, by_deadline, by_parent) = control::reasons();
                if by_file || by_deadline || by_parent {
                    let _ = control::stop_requested();
                    log::event("stop_requested", json!({ "stop_file": by_file, "deadline": by_deadline, "parent_exited": by_parent }));
                    unsafe { PostQuitMessage(0) };
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }
}
