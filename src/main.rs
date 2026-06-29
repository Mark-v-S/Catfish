use crossterm::{
    cursor::SetCursorStyle,
    execute,
    style::Stylize,
    terminal::{Clear, ClearType},
};
use reedline::{
    Emacs, ExampleHighlighter, KeyCode, KeyModifiers, Prompt, PromptEditMode, PromptHistorySearch,
    PromptHistorySearchStatus, ReedlineEvent, Signal, default_emacs_keybindings,
};
use std::{
    borrow::Cow,
    env::{self, current_dir, home_dir},
    fs,
    io::stdout,
    path::Path,
    process::Command,
};
//use walkdir::WalkDir;
mod buildin_commands;
use crate::buildin_commands::{BuildinCMD, cat, echo, ls, mkdir, rm, touch};

/// TOML STUFF
/// ---START---
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct AppConfig {
    prompt: MyPrompt,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct PromptConfig {
    home_symbol: char,
    prompt_symbol: String,
}

/// ---TOML STUFF END---

/// system commands ???
// might be reworking that one for personal satifaction but should work
fn get_path_commands() -> Vec<String> {
    let path = std::env::var("PATH").unwrap_or_default();

    #[cfg(windows)]
    let separator = ';';
    #[cfg(not(windows))]
    let separator = ':';

    let mut commands = Vec::new();

    for dir in path.split(separator) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            // on windows check for .exe, on unix check executable permission
            #[cfg(windows)]
            if entry.path().extension().map_or(false, |e| e == "exe") {
                if let Some(name) = entry.path().file_stem() {
                    commands.push(name.to_string_lossy().to_string());
                }
            }
            #[cfg(not(windows))]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = entry.metadata() {
                    if meta.permissions().mode() & 0o111 != 0 {
                        commands.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    commands
}
//

/// git-stuff
fn get_git_info() -> Option<String> {
    // get current branch
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if !branch.status.success() {
        return None; // not a git repo
    }

    let branch = String::from_utf8(branch.stdout).ok()?;
    let branch = branch.trim();

    // check if there are uncommited changes
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()?;

    let is_dirty = !dirty.stdout.is_empty();

    if is_dirty {
        Some(format!(" ({branch}*) "))
    } else {
        Some(format!(" ({branch}) "))
    }
}
///

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

fn get_curent_path(home_symbol: String) -> String {
    let homedir = get_home_dir();
    let curent_path = env::current_dir().unwrap().to_str().unwrap().to_owned();
    if curent_path.starts_with(&homedir) {
        curent_path.replace(&homedir, &format!("{home_symbol} ").to_string())
    } else {
        curent_path.to_owned()
    }
}
//⛩
// Custom prompt
//
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct MyPrompt {
    prompt_symbol: String,
    home_symbol: String,
    indicator: String,
}

/*
 * the reading prompt config should be in the Prompt functions propably???
 */

impl Prompt for MyPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let top_arrow = "╭╴".magenta();
        let usr = whoami::username()
            .unwrap_or_else(|_| "unknown".into())
            .red();
        let host = whoami::hostname()
            .unwrap_or_else(|_| "unknown".into())
            .cyan();
        let dir = get_curent_path(self.home_symbol.clone()).dark_magenta();
        let bottom_arrow = "╰╴".magenta();
        //let symbol = "ᓚᘏᗢ".dark_grey();
        let symbol = self.prompt_symbol.clone().dark_grey();
        let git = get_git_info().unwrap_or_default();
        Cow::Owned(format!(
            "{top_arrow}{usr} on {host} in {dir}{git}\n{bottom_arrow}{symbol} "
        ))
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("-|")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        let indicator = self.indicator.to_string();
        Cow::Owned(format!("{indicator} "))
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
    let catfish_dir = format!("{}/catfish/", get_home_dir());
    let history_file = format!("{}history.txt", catfish_dir);
    if Path::new(&catfish_dir).exists() == false {
        _ = fs::create_dir(&catfish_dir);
    }

    // TOML STUFF
    // ---START---

    use std::env;

    let args: Vec<String> = env::args().collect();
    use std::fs::File;
    let config: AppConfig;
    let mut config_toml = format!("{}config.toml", catfish_dir);
    if args.contains(&"dev".to_string()) {
        config_toml = format!("config.toml");
        println!("test");
    }
    if Path::new(&config_toml).exists() == true {
        let content = std::fs::read_to_string(config_toml).unwrap();
        config = toml::from_str(&content).unwrap();
    } else {
        config = AppConfig {
            prompt: MyPrompt {
                prompt_symbol: "ᓚᘏᗢ".to_string(),
                home_symbol: "⛩".to_string(),
                indicator: ">".to_string(),
            },
        };
        use std::io::Write;
        let mut file = File::create(&config_toml).expect("Failed to create file");
        let toml_string = toml::to_string(&config).expect("Failed to serialize config");
        file.write_all(toml_string.as_bytes())
            .expect("Failed to write to file");
        _ = fs::create_dir(&catfish_dir);
    }
    // ---END---

    // Use crossterm directly for one-off terminal setup before reedline takes over
    //execute!(stdout(), SetCursorStyle::BlinkingBar)?;
    execute!(stdout(), SetCursorStyle::BlinkingUnderScore)?;

    //let mut prevpath = env::current_dir().unwrap();
    // Set up custom keybindings via reedline's API
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('l'),
        //ReedlineEvent::ExecuteHostCommand("clear".into()), // or handle in match below
        ReedlineEvent::ClearScreen, // or handle in match below
    );

    use reedline::{ColumnarMenu, DefaultCompleter, DefaultHinter, MenuBuilder, ReedlineMenu};
    use reedline::{FileBackedHistory, Reedline};

    let history = Box::new(
        FileBackedHistory::with_file(100, history_file.into())
            .expect("Error configuring history with file"),
    );

    let mut commands = vec![
        "clear".into(),
        "pwd".into(),
        "echo".into(),
        "ls".into(),
        "cat".into(),
        "cd".into(),
        "touch".into(),
        "mkdir".into(),
        "rm".into(),
        "exit".into(),
        "quit".into(),
    ];

    commands.extend(get_path_commands());
    commands.sort();
    commands.dedup(); // remove duplicates

    let highlighter = Box::new(ExampleHighlighter::new(commands.clone()));

    let completer = Box::new(DefaultCompleter::new_with_wordlen(commands.clone(), 2));
    // Use the interactive menu to select options from the completer
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
    // Set up the required keybindings
    //let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));
    let mut line_editor = Reedline::create()
        .with_edit_mode(edit_mode)
        .with_history(history)
        .with_highlighter(highlighter)
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_hinter(Box::new(DefaultHinter::default()));

    // "ᓚᘏᗢ".to_string()
    // "⛩".to_string()
    let prompt = config.prompt;
    /*
    let prompt = MyPrompt::new(
        config.prompt.prompt_symbol,
        config.prompt.home_symbol.to_string(),
        ">".to_string(),
    );
     */

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
                    // list content of path //
                    "ls" => ls(&args),
                    // work with fieles //
                    "cat" => cat(&args),
                    // change current path //
                    "cd" => buildin_cmds.cd(&args),
                    // touch comand //
                    "touch" => touch(&args),
                    // mkdir comand //
                    "mkdir" => mkdir(&args),
                    // rm comand //
                    "rm" => rm(&args),
                    // exit shell //
                    "exit" | "quit" => break,
                    // du nothing and dont break //
                    "" => {}
                    // execute comand on system //
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
