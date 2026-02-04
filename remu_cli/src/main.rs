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
use remu_boot::boot;
use remu_debugger::{DebuggerOption, DebuggerRunner, HarnessPolicy};
use remu_types::TracerDyn;
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
    fn run<P: HarnessPolicy, R: remu_simulator::SimulatorTrait<P, false>>(self, option: DebuggerOption) {
        let tracer: TracerDyn = Rc::new(RefCell::new(CLITracer::new(option.isa.clone())));
        let mut debugger = remu_debugger::Debugger::<P, R>::new(option, tracer);

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
                        if let Err(e) = debugger.execute_line(to_run) {
                            let bt = e.backtrace().map(|s| s.to_string());
                            println!("{:?}", miette::Report::new(e));
                            if let Some(bt) = bt {
                                if bt.is_empty() {
                                    eprintln!("\n(set RUST_BACKTRACE=1 or RUST_LIB_BACKTRACE=1 to capture backtrace)");
                                } else {
                                    eprintln!("\nbacktrace (error site):\n{bt}");
                                }
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

    boot(option, APPRunner);

    Ok(())
}
