use crossterm::{
    cursor::SetCursorStyle,
    execute,
    style::Stylize,
    terminal::{Clear, ClearType},
};
use reedline::{
    Emacs, KeyCode, KeyModifiers, Prompt, PromptEditMode, PromptHistorySearch,
    PromptHistorySearchStatus, Reedline, ReedlineEvent, Signal, default_emacs_keybindings,
};
use std::{
    borrow::Cow,
    env::{self, home_dir},
};
use std::{env::current_dir, io::stdout};

use std::process::Command;
//use walkdir::WalkDir;
mod buildin_commands;
use crate::buildin_commands::{BuildinCMD, cat, echo, ls};

fn execute_command(input: &str) {
    let mut parts = input.trim().split_whitespace();

    let command = match parts.next() {
        Some(cmd) => cmd,
        None => return, // empty input
    };

    let args: Vec<&str> = parts.collect();

    match Command::new(command).args(args).spawn() {
        Ok(mut child) => {
            // wait for the command to finish before showing the prompt again
            let _ = child.wait();
        }
        Err(e) => eprintln!("{e}"),
    }
}

fn get_home_dir() -> String {
    let homedir = home_dir().unwrap().to_str().unwrap().to_owned();
    homedir
}

fn get_curent_path() -> String {
    let homedir = get_home_dir();
    let curent_path = env::current_dir().unwrap().to_str().unwrap().to_owned();
    if curent_path.starts_with(&homedir) {
        curent_path.replace(&homedir, "⛩ ")
    } else {
        curent_path.to_owned()
    }
}

// Custom prompt
struct MyPrompt;

impl Prompt for MyPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let top_arrow = "╭╴".magenta();
        let usr = whoami::username()
            .unwrap_or_else(|_| "unknown".into())
            .red();
        let host = whoami::hostname()
            .unwrap_or_else(|_| "unknown".into())
            .cyan();
        let dir = get_curent_path().dark_magenta();
        let bottom_arrow = "╰╴".magenta();
        let symbol = "ᓚᘏᗢ".dark_grey();
        Cow::Owned(format!(
            "{top_arrow}{usr} on {host} in {dir}\n{bottom_arrow}{symbol} "
        ))
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("-|")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("... ")
    }

    //strg + r
    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        match history_search.status {
            PromptHistorySearchStatus::Passing => {
                Cow::Owned(format!("(search: {}) ", history_search.term))
            }
            PromptHistorySearchStatus::Failing => {
                Cow::Owned(format!("(failed: {}) ", history_search.term))
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use crossterm directly for one-off terminal setup before reedline takes over
    execute!(stdout(), SetCursorStyle::BlinkingBar)?;

    //let mut prevpath = env::current_dir().unwrap();
    // Set up custom keybindings via reedline's API
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('l'),
        ReedlineEvent::ExecuteHostCommand("clear".into()), // or handle in match below
    );

    use reedline::{FileBackedHistory, Reedline};

    let history = Box::new(
        FileBackedHistory::with_file(100, "history.txt".into())
            .expect("Error configuring history with file"),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));
    let mut line_editor = Reedline::create()
        .with_edit_mode(edit_mode)
        .with_history(history);

    let prompt = MyPrompt;

    let curdir = current_dir().unwrap();
    let mut buildin_cmds = BuildinCMD::new(curdir);
    loop {
        let curent_path = env::current_dir().unwrap().to_str().unwrap().to_owned();
        match line_editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                let input = line.trim();
                let mut parts = input.trim().split_whitespace();
                let command = parts.next().unwrap_or("");
                let args: Vec<&str> = parts.collect();
                match command {
                    // Crossterm for manual terminal ops
                    "clear" => execute!(stdout(), Clear(ClearType::All))?,
                    // show curent path //
                    "pwd" => println!("{}", curent_path),
                    // give out curent input //
                    "echo" => echo(&args),
                    "ls" => ls(&args),
                    // work with fieles //
                    "cat" => cat(&args),
                    "cd" => buildin_cmds.cd(&args),
                    "exit" | "quit" => break,
                    "" => {}
                    _ => execute_command(input),
                }
            }
            Ok(Signal::CtrlC) => {
                println!("^C");
            }
            Ok(Signal::CtrlD) => {
                println!("exit");
                break;
            }
            Ok(_) => {
                println!("idk yet men");
            }
            Err(e) => {
                eprintln!("Error: {e}");
                break;
            }
        }
    }

    // Restore cursor on exit
    execute!(stdout(), SetCursorStyle::DefaultUserShape)?;

    Ok(())
}

/*
struct ls{
    path: &'static str,
    hidden: bool,
}

impl ls{
    fn ls_currentpath(Self){

    }
}

struct BuildinCommands;

impl BuildinCommands {
    fn ls(args: &[&str]){}
}
*/
