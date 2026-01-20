use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use nu_ansi_term::{Color, Style};
use reedline::{
    ColumnarMenu, DefaultHinter, DefaultPrompt, DefaultPromptSegment, Emacs, FileBackedHistory,
    KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptEditMode, PromptHistorySearch, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};
use remu_debugger::RemuOptionParer;
use remu_types::TracerDyn;
use std::{borrow::Cow, cell::RefCell, rc::Rc};

remu_macro::mod_flat!(compeleter, highlighter, validator, tracer);

const PROMPT_LEFT: &str = "remu ";
const MULTILINE_PREFIX_LEN: usize = PROMPT_LEFT.len() + 2;

#[derive(Clone)]
struct RemuPrompt {
    inner: DefaultPrompt,
    multiline_prefix_len: usize,
}

impl RemuPrompt {
    fn new(inner: DefaultPrompt, multiline_prefix_len: usize) -> Self {
        Self {
            inner,
            multiline_prefix_len,
        }
    }
}

impl Prompt for RemuPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        self.inner.render_prompt_left()
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        self.inner.render_prompt_right()
    }

    fn render_prompt_indicator(&self, prompt_mode: PromptEditMode) -> Cow<'_, str> {
        self.inner.render_prompt_indicator(prompt_mode)
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        // Align continued lines under the first character after the left prompt.
        Cow::Owned(" ".repeat(self.multiline_prefix_len))
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        self.inner
            .render_prompt_history_search_indicator(history_search)
    }

    fn get_prompt_color(&self) -> reedline::Color {
        self.inner.get_prompt_color()
    }

    fn get_prompt_multiline_color(&self) -> nu_ansi_term::Color {
        self.inner.get_prompt_multiline_color()
    }

    fn get_indicator_color(&self) -> reedline::Color {
        self.inner.get_indicator_color()
    }

    fn get_prompt_right_color(&self) -> reedline::Color {
        self.inner.get_prompt_right_color()
    }

    fn right_prompt_on_last_line(&self) -> bool {
        self.inner.right_prompt_on_last_line()
    }
}

fn get_editor() -> Reedline {
    let history = Box::new(
        FileBackedHistory::with_file(300, "target/cli-history.txt".into())
            .expect("Error configuring history with file"),
    );

    let (graph, root) = remu_debugger::get_command_graph();

    let completer = Box::new(RemuCompleter::new(graph.clone(), root));
    let highlighter = Box::new(RemuHighlighter::new(graph, root));
    // Use the interactive menu to select options from the completer
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
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
        .with_validator(Box::new(RemuValidator::new(MULTILINE_PREFIX_LEN)))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode)
        .with_hinter(Box::new(
            DefaultHinter::default().with_style(Style::new().italic().fg(Color::LightGray)),
        ))
}

fn get_prompt() -> RemuPrompt {
    let inner = DefaultPrompt::new(
        DefaultPromptSegment::Basic(PROMPT_LEFT.into()),
        DefaultPromptSegment::CurrentDateTime,
    );
    RemuPrompt::new(inner, MULTILINE_PREFIX_LEN)
}

fn main() -> Result<()> {
    let _guard = remu_logger::set_logger("target/logs", "remu.log")?;

    let tracer: TracerDyn = Rc::new(RefCell::new(CLITracer::new()));

    let mut line_editor = get_editor();
    let prompt = get_prompt();

    let mut debugger = remu_debugger::Debugger::new(RemuOptionParer::parse(), tracer);

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

    Ok(())
}
