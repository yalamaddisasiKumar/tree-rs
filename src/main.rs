use clap::Parser;
use std::fs;
use std::path::PathBuf;
use glob::Pattern;


/// tree - A simple command-line tool to display directory structures in a tree-like format
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None, help_template = "{bin} {version}\n\n{about}\n\nUSAGE:\n    {usage}\n\nOPTIONS:\n{options}")]
struct TreeArgs {
    #[arg(default_value = ".")]
    custom_path: String,

    /// Whether to display all files, including hidden ones
    #[arg(short)]
    all: bool,

    /// Whether to display only directories
    #[arg(short)]
    directories: bool,

    /// Level of depth to display (default: unlimited)
    #[arg(short = 'L', num_args = 0..=1)]
    level: Option<u16>,

    /// Whether to display full paths instead of just file/directory names
    #[arg(short)]
    full_path: bool,

    /// Filter the output to include only files/directories that match the specified pattern
    #[arg(short = 'P')]
    pattern: Option<String>,

    /// Filter the output to exclude files/directories that match the specified pattern
    #[arg(short = 'I')]
    exclude_pattern: Option<String>,

}

#[derive(Debug)]
struct Config {
    args: TreeArgs,
    pattern_regex: Option<Pattern>,
    exclude_pattern_regex: Option<Pattern>,
}

#[derive(Debug, Clone)]
struct DirectoryEntry {
    name: PathBuf,
    level: u16,
    is_dir: bool,
}
impl DirectoryEntry {

    fn new(path: &PathBuf, level: Option<u16>, is_dir: bool) -> Self {
        DirectoryEntry {
            name: path.to_path_buf(),
            level: level.unwrap_or(0) as u16,
            is_dir,
        }
    }

    fn display_tree(&self, full_path: bool) {
        for _ in 1..self.level {
            print!("│   ");
        }
        if self.level > 0 {
            // print!("+--- ");
            print!("├── ");
        }
        // "└── " for the last entry in a directory, but we don't have that information here
        if full_path {
            println!("{}", self.name.to_string_lossy());
        } else {    
            println!("{}", self.name.file_name().unwrap().to_string_lossy());
        }
    }

}

struct DirectoryTreeIterator<'a > {
    first_time: bool,
    dir : DirectoryEntry,
    stack: Box<std::iter::Flatten<std::vec::IntoIter<DirectoryTreeIterator<'a>>>>,
    config: &'a Config,
}

impl<'a> DirectoryTreeIterator<'a> {
    fn new(root: DirectoryEntry, config: &'a Config) -> Self {
         DirectoryTreeIterator {
            first_time: true,
            dir: root,
            stack: Box::new(Vec::new().into_iter().flatten()),
            config,
        }
    }

    fn set_stack_iterators(&mut self) {
        if self.dir.is_dir && (self.config.args.level.is_none() || self.dir.level < self.config.args.level.unwrap()) {
           if let Ok(y) = fs::read_dir(&self.dir.name) {
                let x = y.filter_map(|res| res.ok())
                    .filter(|d| self.config.args.all || !d.file_name().to_string_lossy().starts_with('.'))
                    .filter(|d| !self.config.args.directories || d.file_type().unwrap().is_dir())
                    .filter_map(|dir_entry|{
                        
                        let de = DirectoryEntry {
                            name: dir_entry.path(),
                            level: self.dir.level + 1,
                            is_dir: dir_entry.file_type().unwrap().is_dir(),
                        };
                    
                        if !de.is_dir && let Some(pattern) = &self.config.pattern_regex {
                            let m = pattern.matches(&dir_entry.file_name().to_string_lossy() );
                            if !m {
                                return None;
                            }
                        }

                        if let Some(pattern) = &self.config.exclude_pattern_regex {
                            let m = pattern.matches(&dir_entry.file_name().to_string_lossy() );
                            if m {
                                return None;
                            }
                        }

                        let dir_itr = DirectoryTreeIterator 
                            ::new( 
                                de,
                                self.config
                            );
                        Some(dir_itr)
                    })
                    .collect::<Vec<DirectoryTreeIterator>>()
                    .into_iter()
                    .flatten();
                self.stack = Box::new(x);
           }
        }
    }
}

impl<'a> Iterator for DirectoryTreeIterator<'a> {
    type Item = DirectoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.first_time {
            self.first_time = false;
            self.set_stack_iterators();
            return Some(self.dir.clone());
        }
        self.stack.next()
    }
}

fn main() {
    let args = TreeArgs::parse();
    let path_buf = if args.custom_path == "." {
        std::env::current_dir().unwrap()
    } else {
        PathBuf::from(args.custom_path.clone())
    };
    let full_path = args.full_path;
    let config = Config {
        args: args.clone(),
        pattern_regex: args.pattern.as_ref().and_then(|p| Pattern::new(p).ok()),
        exclude_pattern_regex: args.exclude_pattern.as_ref().and_then(|p| Pattern::new(p).ok()),
    };

    let dir_iterator = DirectoryTreeIterator::new(
        DirectoryEntry::new(&path_buf, Some(0), true),
        &config
    );
    let mut d = -1;
    let mut f = 0;
    for entry in dir_iterator {
        entry.display_tree(full_path);
        if entry.is_dir {
            d += 1;
        } else {
            f += 1;
        }
    }

    print!("\n{} directories, {} files\n", d, f);

}