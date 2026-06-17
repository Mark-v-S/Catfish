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
use std::io::stdout;
use std::{
    borrow::Cow,
    env::{self, home_dir},
    fs::{File, OpenOptions},
    io::Write,
};

use std::fs;
use std::process::Command;
//use walkdir::WalkDir;

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

    let mut prevpath = env::current_dir().unwrap();
    // Set up custom keybindings via reedline's API
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('l'),
        ReedlineEvent::ExecuteHostCommand("clear".into()), // or handle in match below
    );

    let edit_mode = Box::new(Emacs::new(keybindings));
    let mut line_editor = Reedline::create().with_edit_mode(edit_mode);

    let prompt = MyPrompt;

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

fn ls(args: &[&str]) {
    let mut show_hidden = false;
    let mut path = ".";

    for arg in args {
        match *arg {
            "-a" => show_hidden = true,
            //"-l" => long_format = true,
            other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
            other => path = other,
        }
    }
    buildin_ls(path, show_hidden);
}

fn buildin_ls(path: &str, show_hidden: bool) {
    let mut entries: Vec<_> = match std::fs::read_dir(path) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            eprintln!("ls: {e}");
            return;
        }
    };

    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().to_string();

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        if file_type.is_dir() {
            print!("{}/   ", name.blue());
        } else if file_type.is_file() {
            print!("{}    ", name);
        } else if file_type.is_symlink() {
            print!("{}@   ", name.red());
        }
    }
    println!();
}

fn cat(args: &[&str]) {
    let mut path = ".";
    let mut replace = false;
    let mut append = false;
    let mut numberedlines = false;
    let mut numberedlines_ne = false;
    let mut content = String::from("");

    for arg in args {
        if replace | append {
            match *arg {
                other => path = other,
            }
        } else {
            match *arg {
                "-n" => numberedlines = true,
                "-b" => numberedlines_ne = true,
                ">" => replace = true,
                ">>" => append = true,
                other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
                other => {
                    path = other;
                    let file_contents = fs::read_to_string(path).expect("failed to read");
                    content = format!("{}{}", content, file_contents);
                }
            }
        }
    }
    buildin_cat(
        path,
        content,
        append,
        replace,
        numberedlines,
        numberedlines_ne,
    );
}

fn buildin_cat(
    path: &str,
    content: String,
    append: bool,
    replace: bool,
    numberedlines: bool,
    numberedlines_ne: bool,
) {
    if !append & !replace {
        if !numberedlines & !numberedlines_ne {
            println!("{}", content);
        } else if numberedlines_ne {
            let mut line_num = 0;
            for line in content.lines() {
                if !line.is_empty() {
                    line_num += 1;
                    println!("{:>4}  {line}", line_num);
                } else {
                    println!("      ");
                }
            }
        } else {
            for (i, line) in content.lines().enumerate() {
                println!("{:>4}  {line}", i + 1);
            }
        }
    } else if replace {
        let mut file = File::create(path).expect("Failed to create file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to file");
    } else if append {
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .expect("Failed to open file");
        file.write_all(content.as_bytes())
            .expect("Failed to append to file");
    }
}

fn echo(args: &[&str]) {
    let mut path = ".";
    let mut replace = false;
    let mut append = false;
    let mut content = String::from("");

    for arg in args {
        if replace | append {
            match *arg {
                other => path = other,
            }
        } else {
            match *arg {
                //"-l" => long_format = true,
                ">" => replace = true,
                ">>" => append = true,
                other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
                other => content = format!(r"{} {}", content, other),
            }
        }
    }
    buildin_echo(path, content, append, replace);
}

fn buildin_echo(path: &str, content: String, append: bool, replace: bool) {
    if !append & !replace {
        println!("{}", content);
    } else if replace {
        let mut file = File::create(path).expect("Failed to create file");
        file.write_all(content.as_bytes())
            .expect("Failed to write to file");
    } else if append {
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .expect("Failed to open file");
        file.write_all(content.as_bytes())
            .expect("Failed to append to file");
    }
}

fn cd(args: &[&str], oldpath: &str) {
    let mut path = ".";

    for arg in args {
        match *arg {
            "-" => path = oldpath,
            ".." => path = path,
            //other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
            other => path = other,
        }
    }
    buildin_cd(path);
}

fn buildin_cd(path: &str) {}
