//! `Launcher.exe`: the primary PLC Lab entry point in the installation root.
//!
//! A native Windows window (no console, no web view, no network) that shows the state of
//! the installation and starts the staged application, optionally with a project. All
//! paths are resolved from this executable's folder. Command-line use:
//!
//! - `Launcher.exe` opens the window;
//! - `Launcher.exe <file.plclab>` starts PLC Lab with that project (a running PLC Lab opens
//!   it instead) and exits;
//! - `Launcher.exe --start` starts PLC Lab and exits;
//! - `Launcher.exe --diagnostics` writes the diagnostics report to standard output;
//! - anything else is refused with a usage message (exit code 2).
//!
//! Problems are shown in a message box, or only on standard error when
//! `PLC_LAB_LAUNCHER_NO_DIALOG` is set (automated tests).
#![cfg_attr(windows, windows_subsystem = "windows")]

use plc_lab_launcher::{Installation, check_project_file, data_dir, diagnostics, inspect, start};
use std::{ffi::OsString, path::Path};

const USAGE: &str = "Usage: Launcher.exe [--start | --diagnostics | <project.plclab>]

Without arguments the PLC Lab launcher window opens.";

fn main() {
    let installation = match Installation::of_running_launcher() {
        Ok(i) => i,
        Err(e) => {
            report(&e);
            std::process::exit(1);
        }
    };
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let code = match args.as_slice() {
        [] => window(installation),
        [flag] if flag == "--diagnostics" => {
            let status = inspect(&installation, data_dir().as_deref());
            print!("{}", diagnostics(&installation, &status));
            0
        }
        [flag] if flag == "--start" => outcome(start(&installation, None)),
        [file] if !file.to_string_lossy().starts_with('-') => {
            let path = Path::new(file);
            outcome(check_project_file(path).and_then(|()| start(&installation, Some(path))))
        }
        _ => {
            report(USAGE);
            2
        }
    };
    std::process::exit(code);
}

fn outcome(result: Result<(), String>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(message) => {
            report(&message);
            1
        }
    }
}

/// Show a problem to the user (and on stderr for scripted use).
fn report(message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    if std::env::var_os("PLC_LAB_LAUNCHER_NO_DIALOG").is_none() {
        use winsafe::prelude::Handle;
        let _ = winsafe::HWND::NULL.MessageBox(message, "PLC Lab", winsafe::co::MB::ICONERROR);
    }
}

#[cfg(not(windows))]
fn window(_: Installation) -> i32 {
    report("Launcher.exe shows its window on Windows only; use --start or --diagnostics.");
    1
}

#[cfg(windows)]
fn window(installation: Installation) -> i32 {
    match gui::Launcher::new(installation).and_then(|l| l.run()) {
        Ok(code) => code,
        Err(e) => {
            report(&format!("The launcher window could not be shown: {e}"));
            1
        }
    }
}

#[cfg(windows)]
mod gui {
    use plc_lab_launcher::{
        Check, Installation, Status, VERSION, check_project_file, data_dir, diagnostics, inspect,
        start,
    };
    use std::{path::PathBuf, rc::Rc};
    use winsafe::{self as w, co, gui, prelude::*};

    // Restrained dark industrial palette.
    const BG: (u8, u8, u8) = (24, 28, 34);
    const HEADER: (u8, u8, u8) = (16, 19, 24);
    const ACCENT: (u8, u8, u8) = (63, 167, 214);
    const TEXT: (u8, u8, u8) = (214, 221, 228);
    const MUTED: (u8, u8, u8) = (138, 150, 163);
    const OK: (u8, u8, u8) = (78, 194, 122);
    const WARN: (u8, u8, u8) = (224, 166, 58);
    const BAD: (u8, u8, u8) = (229, 83, 75);

    fn rgb((r, g, b): (u8, u8, u8)) -> w::COLORREF {
        w::COLORREF::from_rgb(r, g, b)
    }
    fn px(v: i32) -> i32 {
        gui::dpi(v, 0).0
    }

    fn color_of(check: &Check) -> (u8, u8, u8) {
        match check {
            Check::Ok(_) => OK,
            Check::Warning(_) => WARN,
            Check::Missing(_) => BAD,
        }
    }
    fn mark(check: &Check) -> &'static str {
        match check {
            Check::Ok(_) => "\u{2713}",
            Check::Warning(_) => "!",
            Check::Missing(_) => "\u{2715}",
        }
    }

    /// Paint a push button in the launcher's palette (native buttons ignore dark themes).
    fn draw_button(p: &w::NMCUSTOMDRAW, primary: bool) -> w::AnyResult<co::CDRF> {
        if p.dwDrawStage != co::CDDS::PREPAINT {
            return Ok(co::CDRF::DODEFAULT);
        }
        let state = p.uItemState;
        let disabled = state.has(co::CDIS::DISABLED);
        let pressed = state.has(co::CDIS::SELECTED);
        let hot = state.has(co::CDIS::HOT);
        let (fill, border, text) = if disabled {
            ((30, 35, 42), (44, 52, 62), (92, 102, 114))
        } else if primary {
            let fill = if pressed {
                (24, 92, 126)
            } else if hot {
                (38, 132, 176)
            } else {
                (31, 114, 154)
            };
            (fill, ACCENT, (245, 248, 250))
        } else {
            let fill = if pressed {
                (28, 33, 40)
            } else if hot {
                (48, 57, 68)
            } else {
                (37, 44, 53)
            };
            (fill, (64, 75, 88), TEXT)
        };
        let brush = w::HBRUSH::GetStockObject(co::STOCK_BRUSH::DC)?;
        p.hdc.SetDCBrushColor(rgb(fill))?;
        p.hdc.FillRect(p.rc, &brush)?;
        let focused = state.has(co::CDIS::FOCUS) && !disabled;
        p.hdc
            .SetDCBrushColor(rgb(if focused { (150, 210, 238) } else { border }))?;
        p.hdc.FrameRect(p.rc, &brush)?;
        p.hdc.SetBkMode(co::BKMODE::TRANSPARENT)?;
        p.hdc.SetTextColor(rgb(text))?;
        let mut bounds = p.rc;
        p.hdc.DrawText(
            &p.hdr.hwndFrom.GetWindowText()?,
            &mut bounds,
            co::DT::CENTER | co::DT::VCENTER | co::DT::SINGLELINE | co::DT::NOPREFIX,
        )?;
        Ok(co::CDRF::SKIPDEFAULT)
    }

    fn font(height: i32, weight: co::FW) -> w::AnyResult<w::guard::DeleteObjectGuard<w::HFONT>> {
        let mut lf = w::LOGFONT::default();
        lf.lfHeight = -px(height);
        lf.lfWeight = weight;
        lf.lfQuality = co::QUALITY::CLEARTYPE;
        lf.set_lfFaceName("Segoe UI");
        Ok(w::HFONT::CreateFontIndirect(&lf)?)
    }

    /// Texts and colors derived from one inspection of the installation.
    struct View {
        status: String,
        status_color: (u8, u8, u8),
        recent: String,
        recent_file: Option<PathBuf>,
        knowledge: String,
        checks: [(String, (u8, u8, u8)); 4],
        startable: bool,
    }

    impl View {
        fn of(status: &Status) -> Self {
            let pack = status.packs.first();
            let application = match &status.application {
                Check::Ok(_) => format!(
                    "Application - PLC Lab Desktop {}",
                    status
                        .release_version
                        .as_deref()
                        .unwrap_or("(version unknown)")
                ),
                _ => "Application - not built: run BUILD_PLC_LAB_RELEASE.bat".into(),
            };
            let knowledge_check = match (&status.knowledge, pack) {
                (Check::Ok(_), Some(p)) => format!("Knowledge pack - {} v{}", p.title, p.version),
                (c, _) => format!("Knowledge pack - {}", c.text()),
            };
            let webview = match &status.webview2 {
                Check::Ok(t) => t.clone(),
                _ => "WebView2 Runtime not found".into(),
            };
            let data = match &status.data {
                Check::Ok(_) => format!(
                    "Project folder - {}",
                    status
                        .data_dir
                        .as_ref()
                        .map_or_else(String::new, |d| d.join("projects").display().to_string())
                ),
                c => format!("Project folder - {}", c.text()),
            };
            let line =
                |check: &Check, text: String| (format!("{}  {text}", mark(check)), color_of(check));
            let summary = status.summary();
            let status_color = if !status.startable() {
                BAD
            } else if summary == "Ready" {
                OK
            } else {
                WARN
            };
            View {
                status: format!("Status: {summary}"),
                status_color,
                recent: match &status.recent {
                    Some(r) if r.exists => r.name.clone(),
                    Some(r) => format!("{} (file no longer exists)", r.name),
                    None => "No project opened yet".into(),
                },
                recent_file: status
                    .recent
                    .as_ref()
                    .filter(|r| r.exists)
                    .map(|r| r.file.clone()),
                knowledge: match pack {
                    Some(p) => format!(
                        "{} v{} \u{00b7} {} files \u{00b7} {}",
                        p.title, p.version, p.files, p.status
                    ),
                    None => "No knowledge pack installed".into(),
                },
                checks: [
                    line(&status.application, application),
                    line(&status.knowledge, knowledge_check),
                    line(&status.webview2, webview),
                    line(&status.data, data),
                ],
                startable: status.startable(),
            }
        }
    }

    #[derive(Clone)]
    pub struct Launcher {
        wnd: gui::WindowMain,
        installation: Rc<Installation>,
        status: Rc<Status>,
        view: Rc<View>,
        fonts: Rc<[w::guard::DeleteObjectGuard<w::HFONT>; 3]>,
        version: gui::Label,
        status_label: gui::Label,
        recent: gui::Label,
        knowledge: gui::Label,
        checks: [gui::Label; 4],
        start: gui::Button,
        open_recent: gui::Button,
        open_project: gui::Button,
        packs: gui::Button,
        diagnostics: gui::Button,
        data_folder: gui::Button,
    }

    const WIDTH: i32 = 540;
    const HEIGHT: i32 = 600;
    const LEFT: i32 = 28;
    const INNER: i32 = WIDTH - 2 * LEFT;

    impl Launcher {
        pub fn new(installation: Installation) -> w::AnyResult<Self> {
            let status = inspect(&installation, data_dir().as_deref());
            let view = View::of(&status);
            let wnd = gui::WindowMain::new(gui::WindowMainOpts {
                title: "PLC Lab",
                class_icon: gui::Icon::Id(32512),
                size: gui::dpi(WIDTH, HEIGHT),
                style: co::WS::CAPTION
                    | co::WS::SYSMENU
                    | co::WS::MINIMIZEBOX
                    | co::WS::CLIPCHILDREN
                    | co::WS::BORDER
                    | co::WS::VISIBLE,
                ..Default::default()
            });
            // Long texts end in "..." (paths shorten in the middle) instead of wrapping.
            let label_with = |text: &str, y: i32, ellipsis: co::SS| {
                gui::Label::new(
                    &wnd,
                    gui::LabelOpts {
                        text,
                        position: gui::dpi(LEFT, y),
                        size: gui::dpi(INNER, 20),
                        control_style: co::SS::LEFT | co::SS::NOPREFIX | ellipsis,
                        ..Default::default()
                    },
                )
            };
            let label = |text: &str, y: i32| label_with(text, y, co::SS::ENDELLIPSIS);
            let button = |text: &str, x: i32, y: i32, width: i32, height: i32, default: bool| {
                gui::Button::new(
                    &wnd,
                    gui::ButtonOpts {
                        text,
                        position: gui::dpi(x, y),
                        width: px(width),
                        height: px(height),
                        control_style: if default {
                            co::BS::DEFPUSHBUTTON
                        } else {
                            co::BS::PUSHBUTTON
                        },
                        ..Default::default()
                    },
                )
            };
            let version = label(&format!("Version {VERSION}"), 112);
            let status_label = label(&view.status, 134);
            let start = button("Start PLC Lab", LEFT, 164, INNER, 40, true);
            let recent = label(&view.recent, 246);
            let open_recent = button("Open Recent", 306, 270, 100, 28, false);
            let open_project = button("Open Project...", 412, 270, 100, 28, false);
            let knowledge = label(&view.knowledge, 332);
            let packs = button("Manage Knowledge Packs", LEFT, 356, 200, 28, false);
            let checks = [
                label(&view.checks[0].0, 424),
                label(&view.checks[1].0, 446),
                label(&view.checks[2].0, 468),
                label_with(&view.checks[3].0, 490, co::SS::PATHELLIPSIS),
            ];
            let diagnostics = button("Diagnostics", LEFT, 524, 140, 30, false);
            let data_folder = button("Open Data Folder", LEFT + 148, 524, 150, 30, false);
            let fonts = Rc::new([
                font(28, co::FW::BOLD)?,
                font(13, co::FW::NORMAL)?,
                font(11, co::FW::SEMIBOLD)?,
            ]);
            let new_self = Self {
                wnd,
                installation: Rc::new(installation),
                status: Rc::new(status),
                view: Rc::new(view),
                fonts,
                version,
                status_label,
                recent,
                knowledge,
                checks,
                start,
                open_recent,
                open_project,
                packs,
                diagnostics,
                data_folder,
            };
            new_self.events();
            Ok(new_self)
        }

        pub fn run(&self) -> w::AnyResult<i32> {
            self.wnd.run_main(None)
        }

        fn buttons(&self) -> [&gui::Button; 6] {
            [
                &self.start,
                &self.open_recent,
                &self.open_project,
                &self.packs,
                &self.diagnostics,
                &self.data_folder,
            ]
        }

        fn events(&self) {
            let me = self.clone();
            self.wnd.on().wm_create(move |_| {
                // Dark title bar (Windows 10 1809 and later; ignored elsewhere).
                let _ = me
                    .wnd
                    .hwnd()
                    .DwmSetWindowAttribute(w::DwmAttr::UseImmersiveDarkMode(true));
                me.start.hwnd().EnableWindow(me.view.startable);
                me.open_project.hwnd().EnableWindow(me.view.startable);
                me.open_recent
                    .hwnd()
                    .EnableWindow(me.view.startable && me.view.recent_file.is_some());
                me.start.hwnd().SetFocus();
                Ok(0)
            });

            let me = self.clone();
            self.wnd.on().wm_paint(move || me.paint());

            let me = self.clone();
            self.wnd.on().wm_ctl_color_static(move |p| {
                let color = if p.hwnd == *me.status_label.hwnd() {
                    me.view.status_color
                } else if p.hwnd == *me.version.hwnd() {
                    MUTED
                } else if p.hwnd == *me.recent.hwnd() {
                    if me.view.recent_file.is_some() {
                        TEXT
                    } else {
                        MUTED
                    }
                } else if p.hwnd == *me.knowledge.hwnd() {
                    if me.status.knowledge.is_ok() {
                        TEXT
                    } else {
                        WARN
                    }
                } else if let Some(i) = me.checks.iter().position(|l| p.hwnd == *l.hwnd()) {
                    me.view.checks[i].1
                } else {
                    TEXT
                };
                p.hdc.SetTextColor(rgb(color))?;
                p.hdc.SetBkColor(rgb(BG))?;
                p.hdc.SetDCBrushColor(rgb(BG))?;
                Ok(w::HBRUSH::GetStockObject(co::STOCK_BRUSH::DC)?)
            });
            for (i, button) in self.buttons().into_iter().enumerate() {
                button.on().nm_custom_draw(move |p| draw_button(p, i == 0));
            }

            let me = self.clone();
            self.start.on().bn_clicked(move || {
                me.launch(None);
                Ok(())
            });
            let me = self.clone();
            self.open_recent.on().bn_clicked(move || {
                match &me.view.recent_file {
                    Some(file) if file.is_file() => me.launch(Some(file.clone())),
                    _ => me.error("The recent project no longer exists."),
                }
                Ok(())
            });
            let me = self.clone();
            self.open_project.on().bn_clicked(move || {
                if let Some(file) = me.choose_project()? {
                    match check_project_file(&file) {
                        Ok(()) => me.launch(Some(file)),
                        Err(e) => me.error(&e),
                    }
                }
                Ok(())
            });
            let me = self.clone();
            self.packs.on().bn_clicked(move || {
                me.open_folder(
                    me.status
                        .data_dir
                        .as_ref()
                        .map(|d| d.join("knowledge-packs")),
                );
                Ok(())
            });
            let me = self.clone();
            self.data_folder.on().bn_clicked(move || {
                me.open_folder(me.status.data_dir.clone());
                Ok(())
            });
            let me = self.clone();
            self.diagnostics.on().bn_clicked(move || {
                let fresh = inspect(&me.installation, data_dir().as_deref());
                DiagnosticsDialog::new(diagnostics(&me.installation, &fresh)).show(&me.wnd)?;
                Ok(())
            });
        }

        fn paint(&self) -> w::AnyResult<()> {
            let hdc = self.wnd.hwnd().BeginPaint()?;
            let client = self.wnd.hwnd().GetClientRect()?;
            let brush = w::HBRUSH::GetStockObject(co::STOCK_BRUSH::DC)?;
            let fill = |rect: w::RECT, color| -> w::AnyResult<()> {
                hdc.SetDCBrushColor(rgb(color))?;
                hdc.FillRect(rect, &brush)?;
                Ok(())
            };
            fill(client, BG)?;
            fill(
                w::RECT {
                    left: 0,
                    top: 0,
                    right: client.right,
                    bottom: px(96),
                },
                HEADER,
            )?;
            fill(
                w::RECT {
                    left: 0,
                    top: px(96),
                    right: client.right,
                    bottom: px(98),
                },
                ACCENT,
            )?;
            for y in [222, 312, 404, 566] {
                fill(
                    w::RECT {
                        left: px(LEFT),
                        top: px(y),
                        right: px(LEFT + INNER),
                        bottom: px(y) + 1,
                    },
                    (44, 52, 62),
                )?;
            }
            hdc.SetBkMode(co::BKMODE::TRANSPARENT)?;
            {
                let _f = hdc.SelectObject(&*self.fonts[0])?;
                hdc.SetTextColor(rgb(TEXT))?;
                hdc.TextOut(px(LEFT), px(18), "PLC LAB")?;
            }
            {
                let _f = hdc.SelectObject(&*self.fonts[1])?;
                hdc.SetTextColor(rgb(MUTED))?;
                hdc.TextOut(
                    px(LEFT + 2),
                    px(60),
                    "Industrial Automation Learning Environment",
                )?;
            }
            {
                let _f = hdc.SelectObject(&*self.fonts[2])?;
                hdc.SetTextColor(rgb(ACCENT))?;
                for (y, title) in [
                    (228, "RECENT PROJECT"),
                    (318, "KNOWLEDGE"),
                    (410, "INSTALLATION"),
                ] {
                    hdc.TextOut(px(LEFT), px(y), title)?;
                }
                hdc.SetTextColor(rgb(MUTED))?;
                hdc.TextOut(
                    px(LEFT),
                    px(574),
                    "\u{00a9} PLC Lab \u{00b7} educational automation simulator \u{00b7} no data leaves this computer",
                )?;
            }
            Ok(())
        }

        fn launch(&self, project: Option<PathBuf>) {
            match start(&self.installation, project.as_deref()) {
                Ok(()) => self.wnd.close(),
                Err(e) => self.error(&e),
            }
        }

        fn error(&self, message: &str) {
            eprintln!("{message}");
            if std::env::var_os("PLC_LAB_LAUNCHER_NO_DIALOG").is_none() {
                let _ = self
                    .wnd
                    .hwnd()
                    .MessageBox(message, "PLC Lab", co::MB::ICONERROR);
            }
        }

        fn open_folder(&self, folder: Option<PathBuf>) {
            let Some(folder) = folder else {
                self.error("The PLC Lab data folder location is unknown.");
                return;
            };
            if let Err(e) = std::fs::create_dir_all(&folder) {
                self.error(&format!("{} cannot be created: {e}", folder.display()));
                return;
            }
            // An explicit user action: show the trusted folder in Explorer.
            if let Err(e) = self.wnd.hwnd().ShellExecute(
                "open",
                &folder.to_string_lossy(),
                None,
                None,
                co::SW::SHOWNORMAL,
            ) {
                self.error(&format!("{} cannot be opened: {e}", folder.display()));
            }
        }

        fn choose_project(&self) -> w::AnyResult<Option<PathBuf>> {
            let _com =
                w::CoInitializeEx(co::COINIT::APARTMENTTHREADED | co::COINIT::DISABLE_OLE1DDE)?;
            let dialog = w::CoCreateInstance::<w::IFileOpenDialog>(
                &co::CLSID::FileOpenDialog,
                None::<&w::IUnknown>,
                co::CLSCTX::INPROC_SERVER,
            )?;
            dialog.SetOptions(
                dialog.GetOptions()? | co::FOS::FORCEFILESYSTEM | co::FOS::FILEMUSTEXIST,
            )?;
            dialog.SetFileTypes(&[("PLC Lab project (*.plclab)", "*.plclab")])?;
            dialog.SetFileTypeIndex(1)?;
            dialog.SetTitle("Open a PLC Lab project")?;
            if !dialog.Show(self.wnd.hwnd())? {
                return Ok(None);
            }
            let path = dialog.GetResult()?.GetDisplayName(co::SIGDN::FILESYSPATH)?;
            Ok(Some(PathBuf::from(path)))
        }
    }

    /// Read-only diagnostics report with "Copy Diagnostics".
    #[derive(Clone)]
    struct DiagnosticsDialog {
        wnd: gui::WindowModal,
        text: Rc<String>,
        edit: gui::Edit,
        copy: gui::Button,
        close: gui::Button,
    }

    impl DiagnosticsDialog {
        fn new(report: String) -> Self {
            let wnd = gui::WindowModal::new(gui::WindowModalOpts {
                title: "PLC Lab diagnostics",
                size: gui::dpi(620, 440),
                ..Default::default()
            });
            let text = report.replace("\r\n", "\n").replace('\n', "\r\n");
            let edit = gui::Edit::new(
                &wnd,
                gui::EditOpts {
                    text: &text,
                    position: gui::dpi(16, 16),
                    width: px(588),
                    height: px(360),
                    control_style: co::ES::MULTILINE | co::ES::READONLY | co::ES::AUTOVSCROLL,
                    window_style: co::WS::CHILD
                        | co::WS::VISIBLE
                        | co::WS::TABSTOP
                        | co::WS::VSCROLL,
                    ..Default::default()
                },
            );
            let button = |text: &str, x: i32| {
                gui::Button::new(
                    &wnd,
                    gui::ButtonOpts {
                        text,
                        position: gui::dpi(x, 392),
                        width: px(140),
                        height: px(30),
                        ..Default::default()
                    },
                )
            };
            let copy = button("Copy Diagnostics", 316);
            let close = button("Close", 464);
            let me = Self {
                wnd,
                text: Rc::new(report),
                edit,
                copy,
                close,
            };
            me.events();
            me
        }

        fn show(&self, parent: &gui::WindowMain) -> w::AnyResult<()> {
            self.wnd.show_modal(parent)
        }

        fn events(&self) {
            let me = self.clone();
            self.wnd.on().wm_create(move |_| {
                let _ = me
                    .wnd
                    .hwnd()
                    .DwmSetWindowAttribute(w::DwmAttr::UseImmersiveDarkMode(true));
                let _ = me.edit.hwnd().SetWindowTheme("DarkMode_Explorer", None);
                Ok(0)
            });
            self.wnd.on().wm_ctl_color_static(move |p| {
                p.hdc.SetTextColor(rgb(TEXT))?;
                p.hdc.SetBkColor(rgb(HEADER))?;
                p.hdc.SetDCBrushColor(rgb(HEADER))?;
                Ok(w::HBRUSH::GetStockObject(co::STOCK_BRUSH::DC)?)
            });
            let me = self.clone();
            self.wnd.on().wm_paint(move || {
                let hdc = me.wnd.hwnd().BeginPaint()?;
                hdc.SetDCBrushColor(rgb(BG))?;
                hdc.FillRect(
                    me.wnd.hwnd().GetClientRect()?,
                    &w::HBRUSH::GetStockObject(co::STOCK_BRUSH::DC)?,
                )?;
                Ok(())
            });
            for button in [&self.copy, &self.close] {
                button.on().nm_custom_draw(|p| draw_button(p, false));
            }
            let me = self.clone();
            self.copy.on().bn_clicked(move || {
                let clipboard = me.wnd.hwnd().OpenClipboard()?;
                clipboard.EmptyClipboard()?;
                let data: Vec<u8> = me
                    .text
                    .encode_utf16()
                    .chain(Some(0))
                    .flat_map(u16::to_le_bytes)
                    .collect();
                clipboard.SetClipboardData(co::CF::UNICODETEXT, &data)?;
                Ok(())
            });
            let me = self.clone();
            self.close.on().bn_clicked(move || {
                me.wnd.close();
                Ok(())
            });
        }
    }
}
