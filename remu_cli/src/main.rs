use anyhow::Result;
use cfonts::{Colors, Fonts, Options, render};
use clap::Parser;
use colored::Colorize;
use nu_ansi_term::{Color, Style};
use reedline::{
    ColumnarMenu, DefaultHinter, Emacs, FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder,
    Reedline, ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};
use remu_simulator::SimulatorOption;
use remu_types::{Rv32I, Rv32IM, RvIsa, TracerDyn};
use std::{cell::RefCell, rc::Rc};
use target_lexicon::{Architecture, Riscv32Architecture};

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

fn run_debugger<I: RvIsa>(option: SimulatorOption) {
    let tracer: TracerDyn = Rc::new(RefCell::new(CLITracer::new(option.isa.clone())));
    let mut debugger = remu_debugger::Debugger::<I>::new(option, tracer);

    let mut line_editor = get_editor();
    let prompt = get_prompt();

    hello();

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                if let Err(e) = debugger.execute_line(buffer) {
                    println!("{:?}", miette::Report::new(e));
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

fn main() -> Result<()> {
    let _guard = remu_logger::set_logger("target/logs", "remu.log")?;

    let option = SimulatorOption::parse();

    match option.isa.0 {
        Architecture::Riscv32(arch) => match arch {
            Riscv32Architecture::Riscv32i => run_debugger::<Rv32I>(option),
            Riscv32Architecture::Riscv32im => run_debugger::<Rv32IM>(option),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }

    Ok(())
}
