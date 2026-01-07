use chrono::{Datelike, Duration, Local, NaiveDate};
use colored::*;
use git2::{Repository, Sort};
use std::collections::HashMap;
use std::env;
use std::path::Path;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| ".".to_string());

    let repo = match Repository::discover(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            std::process::exit(1);
        }
    };

    print_repo_info(&repo);
    println!();
    print_contribution_calendar(&repo);
}

fn print_repo_info(repo: &Repository) {
    let workdir = repo.workdir().unwrap_or(Path::new("."));
    let repo_name = workdir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    // Get branch name
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from))
        .unwrap_or_else(|| "detached".to_string());

    // Count commits
    let commit_count = count_commits(repo);

    // Get contributors
    let contributors = get_contributors(repo);

    // Get languages and LOC
    let code_stats = detect_languages_and_loc(workdir);

    // Get repo size
    let size = get_repo_size(workdir);

    // Print info
    println!("{}", format!("  {}", repo_name).cyan().bold());
    println!("{}", "─".repeat(40).dimmed());
    println!("  {}  {}", "Branch:".white().bold(), branch.yellow());
    println!("  {}  {}", "Commits:".white().bold(), commit_count.to_string().green());
    println!("  {}  {}", "Size:".white().bold(), format_size(size));

    if !contributors.is_empty() {
        let top_contributors: Vec<_> = contributors.iter().take(3).collect();
        println!("  {}  {}", "Authors:".white().bold(),
            top_contributors.iter()
                .map(|(name, count)| format!("{} ({})", name, count))
                .collect::<Vec<_>>()
                .join(", "));
    }

    if !code_stats.languages.is_empty() {
        print!("  {}  ", "LOC:".white().bold());
        let langs: Vec<_> = code_stats.languages.iter().take(5).collect();
        for (i, (lang, _, lines)) in langs.iter().enumerate() {
            let colored_lang = colorize_lang(lang);
            print!("{} ({})", colored_lang, format_number(*lines));
            if i < langs.len() - 1 {
                print!(", ");
            }
        }
        println!();
    }
}

fn colorize_lang(lang: &str) -> ColoredString {
    match lang {
        "Rust" => lang.truecolor(222, 165, 132),       // rust orange
        "Python" => lang.truecolor(55, 118, 171),      // python blue
        "JavaScript" => lang.truecolor(241, 224, 90),  // js yellow
        "TypeScript" => lang.truecolor(49, 120, 198),  // ts blue
        "Go" => lang.truecolor(0, 173, 216),           // go cyan
        "C" => lang.truecolor(85, 85, 85),             // gray
        "C++" => lang.truecolor(243, 75, 125),         // pink
        "Java" => lang.truecolor(176, 114, 25),        // java orange
        "Ruby" => lang.truecolor(204, 52, 45),         // ruby red
        "PHP" => lang.truecolor(119, 123, 180),        // php purple
        "Swift" => lang.truecolor(240, 81, 56),        // swift orange
        "Kotlin" => lang.truecolor(169, 123, 255),     // kotlin purple
        "Scala" => lang.truecolor(220, 50, 47),        // scala red
        "Haskell" => lang.truecolor(94, 80, 134),      // haskell purple
        "OCaml" => lang.truecolor(238, 122, 0),        // ocaml orange
        "Elixir" => lang.truecolor(110, 74, 126),      // elixir purple
        "Erlang" => lang.truecolor(184, 57, 80),       // erlang red
        "Clojure" => lang.truecolor(91, 184, 0),       // clojure green
        "Lua" => lang.truecolor(0, 0, 128),            // lua blue
        "Shell" => lang.truecolor(137, 224, 81),       // shell green
        "Zig" => lang.truecolor(236, 145, 92),         // zig orange
        "Nim" => lang.truecolor(255, 233, 83),         // nim yellow
        "Crystal" => lang.truecolor(0, 0, 0),          // crystal black
        "Vue" => lang.truecolor(65, 184, 131),         // vue green
        "Svelte" => lang.truecolor(255, 62, 0),        // svelte orange
        "React" => lang.truecolor(97, 218, 251),       // react cyan
        "CSS" => lang.truecolor(86, 61, 124),          // css purple
        "HTML" => lang.truecolor(227, 76, 38),         // html orange
        _ => lang.white(),
    }
}

fn count_commits(repo: &Repository) -> usize {
    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return 0,
    };
    revwalk.push_head().ok();
    revwalk.count()
}

fn get_contributors(repo: &Repository) -> Vec<(String, usize)> {
    let mut contributors: HashMap<String, usize> = HashMap::new();

    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    revwalk.push_head().ok();

    for oid in revwalk.filter_map(Result::ok).take(1000) {
        if let Ok(commit) = repo.find_commit(oid) {
            let name = commit.author().name().unwrap_or("Unknown").to_string();
            *contributors.entry(name).or_insert(0) += 1;
        }
    }

    let mut sorted: Vec<_> = contributors.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted
}

struct CodeStats {
    languages: Vec<(String, usize, usize)>, // (name, file_count, lines)
}

fn detect_languages_and_loc(path: &Path) -> CodeStats {
    let mut langs: HashMap<String, (usize, usize)> = HashMap::new(); // (files, lines)

    fn walk_dir(path: &Path, langs: &mut HashMap<String, (usize, usize)>) {
        let Ok(entries) = std::fs::read_dir(path) else { return };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Skip hidden and common non-source dirs
            if name.starts_with('.') || name == "target" || name == "node_modules" || name == "vendor" {
                continue;
            }

            if path.is_dir() {
                walk_dir(&path, langs);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let lang = match ext {
                    "rs" => "Rust",
                    "py" => "Python",
                    "js" => "JavaScript",
                    "ts" => "TypeScript",
                    "go" => "Go",
                    "c" | "h" => "C",
                    "cpp" | "cc" | "cxx" | "hpp" => "C++",
                    "java" => "Java",
                    "rb" => "Ruby",
                    "php" => "PHP",
                    "swift" => "Swift",
                    "kt" => "Kotlin",
                    "scala" => "Scala",
                    "hs" => "Haskell",
                    "ml" | "mli" => "OCaml",
                    "ex" | "exs" => "Elixir",
                    "erl" => "Erlang",
                    "clj" => "Clojure",
                    "lua" => "Lua",
                    "sh" | "bash" => "Shell",
                    "zig" => "Zig",
                    "nim" => "Nim",
                    "cr" => "Crystal",
                    "vue" => "Vue",
                    "svelte" => "Svelte",
                    "jsx" | "tsx" => "React",
                    "css" => "CSS",
                    "html" => "HTML",
                    _ => continue,
                };

                let lines = std::fs::read_to_string(&path)
                    .map(|c| c.lines().count())
                    .unwrap_or(0);

                let entry = langs.entry(lang.to_string()).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += lines;
            }
        }
    }

    walk_dir(path, &mut langs);

    let mut sorted: Vec<_> = langs
        .into_iter()
        .map(|(name, (files, lines))| (name, files, lines))
        .collect();
    sorted.sort_by(|a, b| b.2.cmp(&a.2)); // Sort by lines

    CodeStats { languages: sorted }
}

fn get_repo_size(path: &Path) -> u64 {
    fn dir_size(path: &Path) -> u64 {
        let Ok(entries) = std::fs::read_dir(path) else { return 0 };

        entries.filter_map(Result::ok).fold(0, |acc, entry| {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if name == ".git" || name == "target" || name == "node_modules" {
                return acc;
            }

            if path.is_dir() {
                acc + dir_size(&path)
            } else {
                acc + entry.metadata().map(|m| m.len()).unwrap_or(0)
            }
        })
    }

    dir_size(path)
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn print_contribution_calendar(repo: &Repository) {
    let today = Local::now().date_naive();
    let weeks = 52;
    let start_date = today - Duration::days((weeks * 7) as i64);

    // Adjust to start from Sunday
    let days_since_sunday = start_date.weekday().num_days_from_sunday() as i64;
    let start_date = start_date - Duration::days(days_since_sunday);

    // Collect commits by date
    let mut commits_by_date: HashMap<NaiveDate, usize> = HashMap::new();

    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return,
    };
    revwalk.push_head().ok();
    revwalk.set_sorting(Sort::TIME).ok();

    for oid in revwalk.filter_map(Result::ok) {
        if let Ok(commit) = repo.find_commit(oid) {
            let time = commit.time();
            let date = chrono::DateTime::from_timestamp(time.seconds(), 0)
                .map(|dt| dt.with_timezone(&Local).date_naive());

            if let Some(date) = date {
                if date >= start_date && date <= today {
                    *commits_by_date.entry(date).or_insert(0) += 1;
                }
            }
        }
    }

    // Find max for intensity scaling
    let max_commits = commits_by_date.values().max().copied().unwrap_or(1).max(1);

    // Print month labels
    print!("     ");
    let mut current_month = None;
    for week in 0..weeks {
        let week_start = start_date + Duration::days((week * 7) as i64);
        let month = week_start.month();

        if current_month != Some(month) {
            current_month = Some(month);
            print!("{}", month_abbr(month));
        } else {
            print!("  ");
        }
    }
    println!();

    // Print calendar grid
    let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

    for (day_idx, day_name) in days.iter().enumerate() {
        if day_idx % 2 == 1 {
            print!(" {} ", day_name.dimmed());
        } else {
            print!("     ");
        }

        for week in 0..weeks {
            let date = start_date + Duration::days((week * 7 + day_idx) as i64);

            if date > today {
                print!("  ");
                continue;
            }

            let count = commits_by_date.get(&date).copied().unwrap_or(0);
            let block = get_contribution_block(count, max_commits);
            print!("{} ", block);
        }
        println!();
    }

    // Print legend
    println!();
    print!("     Less ");
    print!("{} ", "█".truecolor(40, 40, 40));
    print!("{} ", "█".truecolor(250, 204, 21));
    print!("{} ", "█".truecolor(251, 146, 60));
    print!("{} ", "█".truecolor(134, 239, 172));
    print!("{} ", "█".truecolor(34, 197, 94));
    println!("More");

    // Print stats
    let total_commits: usize = commits_by_date.values().sum();
    let active_days = commits_by_date.len();
    println!();
    println!("     {} commits in the last year across {} days",
        total_commits.to_string().green().bold(),
        active_days.to_string().cyan());
}

fn get_contribution_block(count: usize, max: usize) -> ColoredString {
    if count == 0 {
        return "█".truecolor(40, 40, 40);
    }

    let intensity = (count as f64 / max as f64 * 4.0).ceil() as usize;

    // Gradient: yellow → orange → light green → green
    match intensity {
        1 => "█".truecolor(250, 204, 21),   // yellow
        2 => "█".truecolor(251, 146, 60),   // orange
        3 => "█".truecolor(134, 239, 172),  // light green
        _ => "█".truecolor(34, 197, 94),    // green
    }
}

fn month_abbr(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "   ",
    }
}
