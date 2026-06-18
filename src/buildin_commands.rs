use crossterm::style::Stylize;
use dirs::home_dir;
use std::{
    env::{current_dir, set_current_dir},
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
};

use std::fs;

pub struct BuildinCMD {
    prevpath: PathBuf,
}

impl BuildinCMD {
    pub fn new(curdir: PathBuf) -> Self {
        Self { prevpath: curdir }
    }
    pub fn cd(&mut self, args: &[&str]) {
        let prevpath = self.prevpath.clone();
        if args.iter().count() > 1 {
            println!("Too many args for cd command");
            return;
        }
        let new_dir = match args.first() {
            None => home_dir().unwrap().to_str().unwrap().to_owned(),
            Some(&"~") => home_dir().unwrap().to_str().unwrap().to_owned(),
            Some(&"-") => prevpath.clone().to_str().unwrap().to_owned(),
            Some(other) => {
                if other.contains("~") {
                    other.replace("~", home_dir().unwrap().to_str().unwrap())
                } else {
                    other.to_string()
                }
            }
        };
        if new_dir == current_dir().unwrap().to_str().unwrap().to_owned() {
            return;
        }
        self.prevpath = current_dir().unwrap();
        if let Err(e) = set_current_dir(new_dir) {
            eprintln!("{}", e);
        }
    }
}

pub fn ls(args: &[&str]) {
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

pub fn cat(args: &[&str]) {
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

pub fn echo(args: &[&str]) {
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

/*
fn cd(args: &[&str], oldpath: &str) {
    let mut path = ".";

    /*
    for arg in args {
        match *arg {
            "-" => path = oldpath,
            ".." => path = path,
            //other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
            other => path = other,
        }
    }
    */
    buildin_cd(path);
}

fn buildin_cd(path: &str) {}
*/
