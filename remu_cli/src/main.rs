use reedline::{DefaultPrompt, FileBackedHistory, Reedline, Signal};

fn main() {
    let prompt = DefaultPrompt::default();

    let history = Box::new(
        FileBackedHistory::with_file(300, "target/cli-history.txt".into())
            .expect("Error configuring history with file"),
    );

    let mut line_editor = Reedline::create().with_history(history);

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
