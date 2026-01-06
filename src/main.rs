#![allow(deprecated)]
use liner::{Completer, Context, CursorPosition, Event, EventKind, FilenameCompleter, Prompt};
use regex::Regex;
use std::env::{self, current_dir};
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use termion::color;

fn highlight_dodo(s: &str) -> String {
    let reg_exp = Regex::new("(?P<k>dodo)").unwrap();
    let format = format!("{}$k{}", color::Fg(color::Red), color::Fg(color::Reset));
    reg_exp.replace_all(s, format.as_str()).to_string()
}

struct CommentCompleter {
    inner: Option<FilenameCompleter>,
}

impl Completer for CommentCompleter {
    fn completions(&mut self, start: &str) -> Vec<String> {
        if let Some(inner) = &mut self.inner {
            inner.completions(start)
        } else {
            Vec::new()
        }
    }

    fn on_event<W: io::Write>(&mut self, event: Event<W>) {
        if let EventKind::BeforeComplete = event.kind {
            let (_, pos) = event.editor.get_words_and_cursor_position();

            // -Figure out of we are completing a command (the first word) or a filename.
            let filename = match pos {
                // -If we are inside of a word(i is the index inside of the text, and if that
                // -position is over zero, we return true
                CursorPosition::InWord(i) => i > 0,
                // -If we are in a space like this `cat | cart` or cat |
                // -checks if there is a word to our left(indicated by there being Some value)
                CursorPosition::InSpace(Some(_), _) => true,
                // -Checks if there is no word to our left(indicated by there being None value)
                CursorPosition::InSpace(None, _) => false,
                // -If we are on the left edge of a word, and the position of the cursor is
                // -greater than or equal to 1, return true
                CursorPosition::OnWordLeftEdge(i) => i >= 1,
                // -If we are on the right edge of the word
                CursorPosition::OnWordRightEdge(i) => i >= 1,
            };

            // -If we are not in a word with pos over zero, or in a space with text beforehand,
            // -or on the left edge of a word with pos >= to 1, or on the Right edge of a word
            // -under the same condition, then
            // -This condition is only false under the predicate that we are in a space with no
            // -word to the left
            self.inner = if filename {
                let completer = FilenameCompleter::new(Some(current_dir().unwrap()));
                Some(completer)
            } else {
                // -Delete the completer
                None
            }
        }
    }
}

fn main() {
    // use signal_hook crate for interacring with SIFINT (to stop closing the shell when exiting a programm) (used help)
    use signal_hook::consts::SIGINT;
    use signal_hook::iterator::Signals;
    let mut signals = Signals::new([SIGINT]).unwrap();
    std::thread::spawn(move || {
        for _ in signals.forever() {
            // do nothing → ignore Ctrl-C in the shell
        }
    });

    // get the home directory as PathBuf and convert to a &str
    let binding = dirs::home_dir().unwrap();
    let homedir = binding.as_os_str().to_str().unwrap();
    // set the path to the history file
    let history_dir = format!("{}/catfish/", homedir);
    let history_file = format!("{}history.txt", history_dir);
    // create variable prevpath for "cd -"
    let mut prevpath = env::current_dir().unwrap();
    // check if history Directory exist if not create it
    if Path::new(&history_dir).exists() == false {
        _ = fs::create_dir(&history_dir);
    }
    // create instanze of Context and CommentCompleter from the "redox_liner" create
    let mut con = Context::new();
    let mut completer = CommentCompleter { inner: None };
    // create the run time history
    con.history
        .set_file_name_and_load_history(history_file)
        .unwrap();
    // the main loop for the chance to do more than command
    loop {
        // get the current directory and convert it to a String
        let mut curpath = env::current_dir()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();

        // replace the home path with a symbol
        if curpath.starts_with(homedir) {
            curpath = curpath.replace(homedir, "⛩"); //ᗢ ⛩ λ | ᓚᘏᗢ
        }

        // configure the upper prompt line
        println!(
            "\x1b[95m╭╴\x1b[0m\x1b[91m{}\x1b[0m on \x1b[96m{}\x1b[0m in \x1b[95m{}\x1b[0m",
            whoami::username().unwrap_or_else(|_| "unknown".into()),
            whoami::hostname().unwrap_or_else(|_| "unknown".into()),
            curpath
        );

        // configure the Prompt
        let prompttext = "\x1b[95m╰\x1b[0m\x1b[26mᓚᘏᗢ \x1b[0m"; //╰ᓚᘏᗢ-> | ╰ᓚᘏᗢ | ╰ᓚᘏᗢ ⛩ (\x1b[91m \x1b[0m)

        // make the prompt with all funktions and reading
        let res = con.read_line(
            Prompt::from(prompttext),
            Some(Box::new(highlight_dodo)),
            &mut completer,
        );

        //catch Error in input (for things like disableing strg + C);
        match res {
            Ok(res) => {
                //res zu string convertiren zur weiter verarbeitung
                let input = res.as_str();

                // split the input at | (pipes) to get the differnt commands
                // -must be peekable so we know when we are on the last command
                let mut commands = input.trim().split(" | ").peekable();
                let mut previous_command = None;

                // check every command
                while let Some(command) = commands.next() {
                    let mut parts = command.trim().split_whitespace();
                    let command = parts.next().unwrap();
                    let args = parts;

                    // check if its one of the "preconfigured" commands and which or not
                    match command {
                        // the cd comand
                        "cd" => {
                            // create the new directory as a string
                            let new_dir = args.peekable().peek().map_or(homedir, |x| *x);
                            // make the new directory a path
                            let root = Path::new(new_dir);
                            // safe the curent directory tempurarly
                            let mut temppath = env::current_dir().unwrap();
                            // if the new directory is the as the working directory set the temporay diractory back to the old directory
                            if (root == temppath) || (new_dir == "~" && homedir == curpath) {
                                temppath = prevpath;
                            // if the new directory is "~" set the directory to the home directory
                            } else if new_dir == "~" {
                                if let Err(e) = env::set_current_dir(homedir) {
                                    eprintln!("{}", e);
                                }
                            // if the new directory is a "-" set the directory to the previous working directory
                            } else if new_dir == "-" {
                                if let Err(e) = env::set_current_dir(prevpath) {
                                    eprintln!("{}", e);
                                }
                            // if the new directory doesnt exist/work give an error and set the temporay diractory back to the old directory
                            } else if let Err(e) = env::set_current_dir(&root) {
                                eprintln!("{}", e);
                                temppath = prevpath;
                            }
                            // set the previous directory to the tempory directory
                            prevpath = temppath;
                            previous_command = None;
                        }
                        // exit the shell
                        "exit" => return,
                        // exicute the command
                        command => {
                            let stdin = previous_command
                                .map_or(Stdio::inherit(), |output: Child| {
                                    Stdio::from(output.stdout.unwrap())
                                });

                            let stdout = if commands.peek().is_some() {
                                // there is another command piped behind this one
                                // prepare to send output to the next command
                                Stdio::piped()
                            } else {
                                // there are no more commands piped behind this one
                                // send output to shell stdout
                                Stdio::inherit()
                            };

                            let output = Command::new(command)
                                .args(args)
                                .stdin(stdin)
                                .stdout(stdout)
                                .spawn();

                            match output {
                                Ok(output) => {
                                    previous_command = Some(output);
                                }
                                Err(e) => {
                                    previous_command = None;
                                    eprintln!("{}", e);
                                }
                            };
                        }
                    }
                }

                // pusch the new command to the runtime history
                con.history.push(input.into()).unwrap();

                if let Some(mut final_command) = previous_command {
                    // -block until the final command has finished
                    let _ = final_command.wait();
                }
                // commit the runtime history to the histoy file
                con.history.commit_to_file();
            }
            Err(e) => {
                match e.kind() {
                    // -ctrl-c pressed
                    io::ErrorKind::Interrupted => {}
                    // -ctrl-d pressed
                    io::ErrorKind::UnexpectedEof => {
                        println!("exiting...");
                        break;
                    }
                    _ => {
                        // -Ensure that all writes to the history file
                        // -are written before exiting due to error.
                        panic!("error: {:?}", e)
                    }
                }
            }
        }
    }
}
