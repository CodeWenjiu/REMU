use anyhow::Result;
use cfonts::{Colors, Fonts, Options, render};
use clap::Parser;
use colored::Colorize;
use nu_ansi_term::{Color, Style};
use reedline::{
    ColumnarMenu, DefaultHinter, Emacs, FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder,
    Reedline, ReedlineEvent, ReedlineMenu, SearchFilter, SearchQuery, Signal,
    default_emacs_keybindings,
};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode as CtKeyCode, KeyEventKind, KeyModifiers as CtKeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use remu_boot::boot;
use remu_debugger::{DebuggerError, DebuggerOption, DebuggerRunner, HarnessPolicy};
use remu_types::TracerDyn;
use std::error::Error;
use std::{cell::RefCell, rc::Rc};

remu_macro::mod_flat!(compeleter, highlighter, validator, prompt, tracer);

fn get_editor() -> Reedline {
    let history = Box::new(
        FileBackedHistory::with_file(300, "target/cli-history.txt".into())
            .expect("Error configuring history with file"),
    );

    let (graph, root) = remu_debugger::get_command_graph();

    let completer = Box::new(RemuCompleter::new(graph.clone(), root));
    let highlighter = Box::new(RemuHighlighter::new(graph, root));
    // Use the interactive menu to select options from the completer
    let completion_menu = Box::new(
        ColumnarMenu::default()
            .with_name("completion_menu")
            .with_columns(8)
            .with_column_width(None)
            .with_column_padding(0),
    );
    // Set up the required keybindings
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::SHIFT,
        KeyCode::BackTab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuPrevious,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    Reedline::create()
        .with_history(history)
        .with_highlighter(highlighter)
        .with_completer(completer)
        .with_quick_completions(true)
        .with_validator(Box::new(RemuValidator::new(PROMPT_LEN)))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode)
        .with_hinter(Box::new(
            DefaultHinter::default().with_style(Style::new().italic().fg(Color::LightGray)),
        ))
}

/// Runs `f` while a background thread polls for Ctrl+C and sets `interrupt` when seen.
/// CLI-internal: how we feed interrupt during long-running commands (e.g. continue/step).
fn run_with_interrupt_polling<R, F>(interrupt: &Arc<AtomicBool>, f: F) -> R
where
    F: FnOnce() -> R,
{
    let running = Arc::new(AtomicBool::new(true));
    let thread_handle = thread::spawn({
        let running = Arc::clone(&running);
        let interrupt = Arc::clone(interrupt);
        move || {
            if enable_raw_mode().is_err() {
                return;
            }
            while running.load(Ordering::Relaxed) {
                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(Event::Key(key)) = event::read() {
                        if key.kind != KeyEventKind::Press {
                            continue;
                        }
                        let is_ctrl_c = match key.code {
                            CtKeyCode::Char('c') | CtKeyCode::Char('C') => {
                                key.modifiers.contains(CtKeyModifiers::CONTROL)
                            }
                            CtKeyCode::Char(c) if c == '\x03' => true,
                            _ => false,
                        };
                        if is_ctrl_c {
                            interrupt.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                }
            }
            let _ = disable_raw_mode();
        }
    });
    let result = f();
    running.store(false, Ordering::Relaxed);
    let _ = thread_handle.join();
    result
}

fn hello() {
    let output = render(Options {
        text: String::from("remu"),
        font: Fonts::FontSimple,
        colors: vec![Colors::Yellow],
        ..Options::default()
    });

    println!();
    println!("{}", "welcome to".magenta());
    println!("{}", output.text);
}

struct APPRunner;

impl DebuggerRunner for APPRunner {
    fn run<P: HarnessPolicy, R: remu_simulator::SimulatorTrait<P, false>>(
        self,
        option: DebuggerOption,
        interrupt: Arc<AtomicBool>,
    ) {
        let tracer: TracerDyn = Rc::new(RefCell::new(CLITracer::new(option.isa.clone())));
        let interrupt_for_polling = Arc::clone(&interrupt);

        let mut debugger = match remu_debugger::Debugger::<P, R>::new(option, tracer, interrupt) {
            Ok(d) => d,
            Err(DebuggerError::ExitRequested) => {
                println!("{}", "Quiting...".cyan());
                return;
            }
            Err(e) => {
                eprintln!("startup: {}", e);
                return;
            }
        };

        let mut line_editor = get_editor();
        let prompt = get_prompt();

        hello();

        loop {
            let sig = line_editor.read_line(&prompt);
            match sig {
                Ok(Signal::Success(buffer)) => {
                    let to_run = if buffer.trim().is_empty() {
                        line_editor
                            .history()
                            .search(SearchQuery::last_with_search(SearchFilter::anything(
                                line_editor.get_history_session_id(),
                            )))
                            .ok()
                            .and_then(|v| v.into_iter().next())
                            .map(|h| h.command_line)
                            .unwrap_or(buffer)
                    } else {
                        buffer
                    };
                    if !to_run.trim().is_empty() {
                        let result = run_with_interrupt_polling(&interrupt_for_polling, || debugger.execute_line(to_run));
                        if let Err(e) = result {
                            if matches!(&e, DebuggerError::ExitRequested) {
                                println!("{}", "Quiting...".cyan());
                                break;
                            }
                            eprintln!("{}", e);
                            let mut src: Option<&(dyn Error + 'static)> = e.source();
                            while let Some(s) = src {
                                eprintln!("  caused by: {}", s);
                                src = s.source();
                            }
                            if let Some(bt) = e.backtrace() {
                                eprintln!("\nStack backtrace:\n{}", bt);
                            }
                        }
                    }
                }
                Ok(Signal::CtrlD) => {
                    println!("{}", "Quiting...".cyan());
                    break;
                }
                _ => {}
            }
        }
    }
}

fn main() -> Result<()> {
    let _guard = remu_logger::set_logger("target/logs", "remu.log")?;

    let option = DebuggerOption::parse();

    let interrupt = Arc::new(AtomicBool::new(false));
    boot(option, APPRunner, interrupt);

    Ok(())
}
