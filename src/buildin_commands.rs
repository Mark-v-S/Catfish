use chrono::{DateTime, Utc};
use chrono::{Datelike, Month, Timelike};
use crossterm::style::{StyledContent, Stylize};
use dirs::home_dir;
use filetime::set_file_times;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use std::{
    env::{current_dir, set_current_dir},
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
};

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
    let mut long_format = false;
    let mut path = ".";

    for arg in args {
        match *arg {
            "-a" => show_hidden = true,
            "-l" => long_format = true,
            other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
            other => path = other,
        }
    }
    buildin_ls(path, show_hidden, long_format);
}

fn buildin_ls(path: &str, show_hidden: bool, long_format: bool) {
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
        let mut name = entry.file_name().to_string_lossy().to_string();
        let modified = entry.metadata().unwrap().modified().unwrap();
        let len = entry.metadata().unwrap().len();
        let datetime: DateTime<Utc> = modified.into();
        let sname: StyledContent<String>;

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        if file_type.is_dir() {
            name = format!("{}/", name);
            sname = name.blue();
        } else if file_type.is_file() {
            sname = name.bold();
        } else if file_type.is_symlink() {
            name = format!("{}@", name);
            sname = name.red();
        } else {
            sname = name.bold();
        }

        if !long_format {
            print!("{}  ", sname);
        }
        if long_format {
            #[cfg(unix)]
            {
                let permissions = entry.metadata().unwrap().permissions();
                use std::os::unix::fs::PermissionsExt;
                println!(
                    "{:<5} {:>10} {}  {}    ",
                    format_permissions(permissions.mode()),
                    len,
                    datetime.format("%d. %b %H:%M").to_string(),
                    sname
                );
            }

            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;

                let attrs = entry.metadata().unwrap().file_attributes();
                let type_char = if file_type.is_dir() { 'd' } else { '-' };
                let a = if attrs & 0x20 != 0 { 'a' } else { '-' }; // archive
                let r = if attrs & 0x1 != 0 { 'r' } else { '-' }; // readonly
                let h = if attrs & 0x2 != 0 { 'h' } else { '-' }; // hidden
                let s = if attrs & 0x4 != 0 { 's' } else { '-' }; // system
                let l = if file_type.is_symlink() { 'l' } else { '-' };

                let file_attribues = format!("{type_char}{a}{r}{h}{s}{l}");
                println!(
                    "{:<5} {:>10} {}  {}    ",
                    file_attribues,
                    len,
                    datetime.format("%d. %b %H:%M").to_string(),
                    sname
                );
            }
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

pub fn touch(args: &[&str]) {
    let mut files: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            //"-n" => numberedlines = true,
            //"-b" => numberedlines_ne = true,
            other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
            other => {
                files.push(other);
            }
        }
    }
    buildin_touch(files);
}

fn buildin_touch(files: Vec<&str>) {
    for file in files {
        let path = Path::new(file);
        if path.exists() {
            use filetime::FileTime;
            let time = FileTime::now();
            set_file_times(file, time, time).expect("faild to file time");
        } else {
            let _file = File::create(file).expect("Failed to create file");
        }
    }
}

pub fn mkdir(args: &[&str]) {
    let mut files: Vec<&str> = Vec::new();
    let mut nested = false;

    for arg in args {
        match *arg {
            //"-n" => numberedlines = true,
            "-p" => nested = true,
            other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
            other => {
                files.push(other);
            }
        }
    }
    buildin_mkdir(files, nested);
}

fn buildin_mkdir(files: Vec<&str>, nested: bool) {
    for file in files {
        let path = Path::new(file);
        if nested {
            fs::create_dir_all(path).expect("faild to create nested folders");
        } else {
            fs::create_dir(path).expect("faild to create folder");
        }
    }
}

pub fn rm(args: &[&str]) {
    let mut files: Vec<&str> = Vec::new();

    for arg in args {
        match *arg {
            //"-n" => numberedlines = true,
            //"-b" => numberedlines_ne = true,
            other if other.starts_with('-') => eprintln!("ls: unknown flag {other}"),
            other => {
                files.push(other);
            }
        }
    }
    buildin_rm(files);
}

fn buildin_rm(files: Vec<&str>) {
    for file in files {
        let path = Path::new(file);
        match fs::remove_file(path) {
            Ok(_) => println!("File deleted successfully."),
            Err(e) => println!("Error deleting file: {}", e),
        }
        /*
        if path.exists() {
            use filetime::FileTime;
            let time = FileTime::now();
            set_file_times(file, time, time).expect("faild to file time");
        } else {
            let _file = File::create(file).expect("Failed to create file");
        }*/
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

#[cfg(unix)]
fn format_permissions(mode: u32) -> String {
    let chars = ['x', 'w', 'r'];
    let mut result = String::with_capacity(9);

    for i in (0..3).rev() {
        let bits = (mode >> (i * 3)) & 0b111;
        result.push(if bits & 0b100 != 0 { 'r' } else { '-' });
        result.push(if bits & 0b010 != 0 { 'w' } else { '-' });
        result.push(if bits & 0b001 != 0 { 'x' } else { '-' });
    }

    result
}
