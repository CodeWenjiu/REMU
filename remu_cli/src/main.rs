use nu_ansi_term::{Color, Style};
use reedline::{
    ColumnarMenu, DefaultCompleter, DefaultHinter, DefaultPrompt, DefaultPromptSegment, Emacs,
    ExampleHighlighter, FileBackedHistory, KeyCode, KeyModifiers, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};

fn get_editor() -> Reedline {
    let history = Box::new(
        FileBackedHistory::with_file(300, "target/cli-history.txt".into())
            .expect("Error configuring history with file"),
    );

    let commands = vec![
        "test".into(),
        "hello world".into(),
        "hello world reedline".into(),
        "this is the reedline crate".into(),
    ];

    let completer = Box::new(DefaultCompleter::new_with_wordlen(commands.clone(), 2));
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

    let edit_mode = Box::new(Emacs::new(keybindings));

    Reedline::create()
        .with_history(history)
        .with_highlighter(Box::new(ExampleHighlighter::new(commands)))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode)
        .with_hinter(Box::new(
            DefaultHinter::default().with_style(Style::new().italic().fg(Color::LightGray)),
        ))
}

fn get_prompt() -> DefaultPrompt {
    DefaultPrompt::new(
        DefaultPromptSegment::Basic("(remu)".into()),
        DefaultPromptSegment::CurrentDateTime,
    )
}

fn main() {
    let mut line_editor = get_editor();
    let prompt = get_prompt();

    loop {
        let sig = line_editor.read_line(&prompt);
        match sig {
            Ok(Signal::Success(buffer)) => {
                println!("We processed: {}", buffer);
            },
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => {
                println!("\nAborted!");
                break;
            },
            x => {
                println!("Event: {:?}", x);
            },
        }
    }
}
