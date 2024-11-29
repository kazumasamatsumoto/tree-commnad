use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// ファイル階層と責務をツリー状に表示するCLIツール
#[derive(Parser)]
#[command(
    author,
    version,
    about,
    long_about = None,
    args_conflicts_with_subcommands = true
)]
struct Cli {
    /// サブコマンド
    #[command(subcommand)]
    command: Option<Commands>,

    /// 調査するディレクトリのパス
    #[arg(default_value = ".", index = 1)]
    path: String,
}

#[derive(Subcommand)]
enum Commands {
    /// 自動補完スクリプトを生成
    Completion {
        /// 生成するシェルの種類 (bash, zsh, fish, powershell, elvish)
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Some(Commands::Completion { shell }) = cli.command {
        let mut cmd = Cli::command();
        let bin_name = cmd.get_name().to_string();
        generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
        return;
    }

    // パスを絶対パスに変換
    let target_dir = match Path::new(&cli.path).canonicalize() {
        Ok(path) => path,
        Err(e) => {
            eprintln!("Error: Could not access target directory: {}", e);
            std::process::exit(1);
        }
    };

    let entries = collect_entries(&target_dir);
    print_tree(&entries, &target_dir, &mut Vec::new());
}

fn collect_entries(target_dir: &Path) -> HashMap<PathBuf, Vec<DirEntry>> {
    let mut entries: HashMap<PathBuf, Vec<DirEntry>> = HashMap::new();

    // ルートディレクトリをエントリに追加
    entries.entry(target_dir.to_path_buf()).or_default();

    for entry in WalkDir::new(target_dir)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(Result::ok)
    {
        let _path = entry.path().to_path_buf();
        let parent = entry.path().parent().unwrap().to_path_buf();
        entries.entry(parent).or_default().push(entry);
    }

    // 各ディレクトリ内のエントリをソート
    for vec in entries.values_mut() {
        vec.sort_by(|a, b| {
            a.file_type()
                .is_dir()
                .cmp(&b.file_type().is_dir())
                .reverse()
                .then_with(|| {
                    let a_name = a.file_name().to_string_lossy().to_lowercase();
                    let b_name = b.file_name().to_string_lossy().to_lowercase();
                    a_name.cmp(&b_name)
                })
        });
    }

    entries
}

fn print_tree(entries: &HashMap<PathBuf, Vec<DirEntry>>, path: &Path, prefix: &mut Vec<bool>) {
    if let Some(children) = entries.get(path) {
        let count = children.len();
        for (i, entry) in children.iter().enumerate() {
            let is_last = i == count - 1;
            let file_name = entry.file_name().to_string_lossy();
            let mut line_prefix = String::new();
            for &last in prefix.iter() {
                if last {
                    line_prefix.push_str("    ");
                } else {
                    line_prefix.push_str("│   ");
                }
            }
            if is_last {
                line_prefix.push_str("└── ");
            } else {
                line_prefix.push_str("├── ");
            }

            if entry.path().is_dir() {
                println!("{}{}", line_prefix, file_name);
                prefix.push(is_last);
                print_tree(entries, &entry.path(), prefix);
                prefix.pop();
            } else {
                let responsibility = get_responsibility(&entry.path());
                println!("{}{} - {}", line_prefix, file_name, responsibility);
            }
        }
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn get_responsibility(path: &Path) -> String {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                let trimmed_line = line.trim();
                if !trimmed_line.is_empty() {
                    if trimmed_line.starts_with("//") || trimmed_line.starts_with("#") {
                        return trimmed_line
                            .trim_start_matches(|c: char| c.is_whitespace() || c == '/' || c == '#')
                            .to_string();
                    } else {
                        break;
                    }
                }
            }
        }
    }
    "No responsibility comment".to_string()
}
