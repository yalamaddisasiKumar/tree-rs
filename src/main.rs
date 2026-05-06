use clap::Parser;
use std::fs;
use std::io::{self, BufWriter, Write};
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

    fn display_tree(&self, full_path: bool, out: &mut impl Write) {
        for _ in 1..self.level {
            write!(out, "│   ").unwrap();
        }
        if self.level > 0 {
            write!(out, "├── ").unwrap();
        }
        // "└── " for the last entry in a directory, but we don't have that information here
        if full_path {
            writeln!(out, "{}", self.name.to_string_lossy()).unwrap();
        } else {
            writeln!(out, "{}", self.name.file_name().unwrap().to_string_lossy()).unwrap();
        }
    }

}

struct DirectoryTreeIterator<'a> {
    first_time: bool,
    dir: DirectoryEntry,
    stack: Box<dyn Iterator<Item = DirectoryEntry> + 'a>,
    config: &'a Config,
}

impl<'a> DirectoryTreeIterator<'a> {
    fn new(root: DirectoryEntry, config: &'a Config) -> Self {
        DirectoryTreeIterator {
            first_time: true,
            dir: root,
            stack: Box::new(std::iter::empty()),
            config,
        }
    }

    fn set_stack_iterators(&mut self) {
        if self.dir.is_dir && (self.config.args.level.is_none() || self.dir.level < self.config.args.level.unwrap()) {
            if let Ok(y) = fs::read_dir(&self.dir.name) {
                let config = self.config;
                let level = self.dir.level;
                let x = y.filter_map(|res| res.ok())
                    .filter(move |d| config.args.all || !d.file_name().to_string_lossy().starts_with('.'))
                    .filter_map(move |dir_entry| {
                        let is_dir = dir_entry.file_type().ok()?.is_dir();
                        if config.args.directories && !is_dir {
                            return None;
                        }
                        let de = DirectoryEntry {
                            name: dir_entry.path(),
                            level: level + 1,
                            is_dir,
                        };

                        if !de.is_dir && let Some(pattern) = &config.pattern_regex {
                            if !pattern.matches(&dir_entry.file_name().to_string_lossy()) {
                                return None;
                            }
                        }

                        if let Some(pattern) = &config.exclude_pattern_regex {
                            if pattern.matches(&dir_entry.file_name().to_string_lossy()) {
                                return None;
                            }
                        }

                        Some(DirectoryTreeIterator::new(de, config))
                    })
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
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut d = -1;
    let mut f = 0;
    for entry in dir_iterator {
        entry.display_tree(full_path, &mut out);
        if entry.is_dir {
            d += 1;
        } else {
            f += 1;
        }
    }

    writeln!(out, "\n{} directories, {} files", d, f).unwrap();

}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn make_args() -> TreeArgs {
        TreeArgs {
            custom_path: ".".to_string(),
            all: false,
            directories: false,
            level: None,
            full_path: false,
            pattern: None,
            exclude_pattern: None,
        }
    }

    fn make_config(args: TreeArgs) -> Config {
        let pattern_regex = args.pattern.as_ref().and_then(|p| Pattern::new(p).ok());
        let exclude_pattern_regex = args.exclude_pattern.as_ref().and_then(|p| Pattern::new(p).ok());
        Config { args, pattern_regex, exclude_pattern_regex }
    }

    /// Collect file_name strings for all entries beneath the root (root excluded).
    fn collect_names(root: PathBuf, config: &Config) -> Vec<String> {
        let root_entry = DirectoryEntry::new(&root, Some(0), true);
        DirectoryTreeIterator::new(root_entry, config)
            .skip(1)
            .map(|e| e.name.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    // ── display_tree ─────────────────────────────────────────────────────────

    #[test]
    fn display_root_has_no_prefix() {
        let entry = DirectoryEntry { name: PathBuf::from("mydir"), level: 0, is_dir: true };
        let mut buf = Vec::new();
        entry.display_tree(false, &mut buf);
        assert_eq!(String::from_utf8(buf).unwrap(), "mydir\n");
    }

    #[test]
    fn display_level_1_has_branch() {
        let entry = DirectoryEntry {
            name: PathBuf::from("root").join("file.txt"),
            level: 1,
            is_dir: false,
        };
        let mut buf = Vec::new();
        entry.display_tree(false, &mut buf);
        assert_eq!(String::from_utf8(buf).unwrap(), "├── file.txt\n");
    }

    #[test]
    fn display_level_2_has_pipe_indent_and_branch() {
        let entry = DirectoryEntry {
            name: PathBuf::from("root").join("sub").join("file.txt"),
            level: 2,
            is_dir: false,
        };
        let mut buf = Vec::new();
        entry.display_tree(false, &mut buf);
        assert_eq!(String::from_utf8(buf).unwrap(), "│   ├── file.txt\n");
    }

    #[test]
    fn display_full_path_prints_whole_path() {
        let name = PathBuf::from("root").join("file.txt");
        let entry = DirectoryEntry { name: name.clone(), level: 1, is_dir: false };
        let mut buf = Vec::new();
        entry.display_tree(true, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains(&name.to_string_lossy().to_string()));
    }

    // ── iterator – basic structure ────────────────────────────────────────────

    #[test]
    fn iterator_first_item_is_root() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(make_args());
        let root = DirectoryEntry::new(&tmp.path().to_path_buf(), Some(0), true);
        let first = DirectoryTreeIterator::new(root, &config).next().unwrap();
        assert_eq!(first.level, 0);
        assert!(first.is_dir);
    }

    #[test]
    fn iterator_empty_dir_yields_only_root() {
        let tmp = TempDir::new().unwrap();
        let config = make_config(make_args());
        let root = DirectoryEntry::new(&tmp.path().to_path_buf(), Some(0), true);
        let entries: Vec<_> = DirectoryTreeIterator::new(root, &config).collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn iterator_lists_direct_children() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "").unwrap();
        fs::write(tmp.path().join("b.txt"), "").unwrap();
        let config = make_config(make_args());
        let mut names = collect_names(tmp.path().to_path_buf(), &config);
        names.sort();
        assert_eq!(names, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn iterator_recurses_into_subdirectory() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub").join("inner.txt"), "").unwrap();
        let config = make_config(make_args());
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(names.contains(&"sub".to_string()));
        assert!(names.contains(&"inner.txt".to_string()));
    }

    #[test]
    fn entry_levels_are_correct_for_nested_path() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub").join("file.txt"), "").unwrap();
        let config = make_config(make_args());
        let root = DirectoryEntry::new(&tmp.path().to_path_buf(), Some(0), true);
        let entries: Vec<_> = DirectoryTreeIterator::new(root, &config).collect();
        let sub = entries.iter().find(|e| e.name.file_name().unwrap() == "sub").unwrap();
        let file = entries.iter().find(|e| e.name.file_name().unwrap() == "file.txt").unwrap();
        assert_eq!(sub.level, 1);
        assert_eq!(file.level, 2);
    }

    // ── hidden file filter ────────────────────────────────────────────────────

    #[test]
    fn hidden_files_excluded_by_default() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let config = make_config(make_args());
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(!names.contains(&".hidden".to_string()));
        assert!(names.contains(&"visible.txt".to_string()));
    }

    #[test]
    fn hidden_files_shown_with_all_flag() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".hidden"), "").unwrap();
        fs::write(tmp.path().join("visible.txt"), "").unwrap();
        let mut args = make_args();
        args.all = true;
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(names.contains(&".hidden".to_string()));
        assert!(names.contains(&"visible.txt".to_string()));
    }

    // ── directories-only filter ───────────────────────────────────────────────

    #[test]
    fn directories_flag_excludes_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "").unwrap();
        fs::create_dir(tmp.path().join("subdir")).unwrap();
        let mut args = make_args();
        args.directories = true;
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(!names.contains(&"file.txt".to_string()));
        assert!(names.contains(&"subdir".to_string()));
    }

    // ── depth limit ───────────────────────────────────────────────────────────

    #[test]
    fn depth_limit_stops_at_specified_level() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("lvl1")).unwrap();
        fs::create_dir(tmp.path().join("lvl1").join("lvl2")).unwrap();
        fs::write(tmp.path().join("lvl1").join("lvl2").join("deep.txt"), "").unwrap();
        let mut args = make_args();
        args.level = Some(1);
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(names.contains(&"lvl1".to_string()));
        assert!(!names.contains(&"lvl2".to_string()));
        assert!(!names.contains(&"deep.txt".to_string()));
    }

    #[test]
    fn depth_limit_zero_yields_only_root() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file.txt"), "").unwrap();
        let mut args = make_args();
        args.level = Some(0);
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(names.is_empty());
    }

    // ── include pattern ───────────────────────────────────────────────────────

    #[test]
    fn include_pattern_keeps_matching_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.rs"), "").unwrap();
        fs::write(tmp.path().join("readme.txt"), "").unwrap();
        let mut args = make_args();
        args.pattern = Some("*.rs".to_string());
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(names.contains(&"main.rs".to_string()));
        assert!(!names.contains(&"readme.txt".to_string()));
    }

    #[test]
    fn include_pattern_does_not_filter_directories() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("mydir")).unwrap();
        fs::write(tmp.path().join("skip.txt"), "").unwrap();
        let mut args = make_args();
        args.pattern = Some("*.rs".to_string()); // no .rs files present
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(names.contains(&"mydir".to_string()));
        assert!(!names.contains(&"skip.txt".to_string()));
    }

    // ── exclude pattern ───────────────────────────────────────────────────────

    #[test]
    fn exclude_pattern_removes_matching_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("keep.rs"), "").unwrap();
        fs::write(tmp.path().join("remove.txt"), "").unwrap();
        let mut args = make_args();
        args.exclude_pattern = Some("*.txt".to_string());
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(names.contains(&"keep.rs".to_string()));
        assert!(!names.contains(&"remove.txt".to_string()));
    }

    #[test]
    fn exclude_pattern_removes_matching_directories() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join("node_modules")).unwrap();
        fs::write(tmp.path().join("node_modules").join("pkg.js"), "").unwrap();
        fs::create_dir(tmp.path().join("src")).unwrap();
        let mut args = make_args();
        args.exclude_pattern = Some("node_modules".to_string());
        let config = make_config(args);
        let names = collect_names(tmp.path().to_path_buf(), &config);
        assert!(!names.contains(&"node_modules".to_string()));
        assert!(!names.contains(&"pkg.js".to_string()));
        assert!(names.contains(&"src".to_string()));
    }
}