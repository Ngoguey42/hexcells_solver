use reddit_post;
use solver;
use std::fs::File;
use std::io::Write;

pub enum Outcome {
    ParseFail,
    Solver(solver::Outcome),
}

pub struct Line {
    pub post: reddit_post::RedditPost,
    pub idx_in_post: u32,
    pub level_name: String,
    pub outcome: Outcome,
}

const HEADER0: &str = "Classif,Upvotes,Date,Author,Post,Title,URL\n";
const HEADER1: &str = "Difficulty,Upvotes,Date,Author,Post,Title,URL\n";

fn cleanup_post_name(s: &str) -> String {
    let s = s
        .replace("\"", "'")
        .replace("[level]", "")
        .replace("[Level]", "")
        .replace("[Level Pack]", "")
        .replace("[Level-Pack]", "")
        .replace("[Levle pack]", "");
    let mut s = s.trim().to_string();
    if s.len() > 40 {
        s.truncate(34);
        s = format!("{} [...]", s);
        println!("> {}", s);
    }
    s.to_string()
}

pub fn report_all(lines: &Vec<Line>) {
    let mut report_lines: Vec<String> = vec![];
    for line in lines {
        let post = &line.post;
        let classif = match &line.outcome {
            Outcome::ParseFail => "Err".to_string(),
            Outcome::Solver(solver::Outcome::Timeout) => "T".to_string(),
            Outcome::Solver(solver::Outcome::Unsolvable) => "Spe".to_string(),
            Outcome::Solver(solver::Outcome::Solved(findings_vec)) => {
                let (max_local, max_global) = solver::difficulty_of_findings_vec(&findings_vec);
                match (max_local, max_global) {
                    (None, None) => panic!(),
                    (Some(i), None) => format!("{}", i),
                    (Some(i), Some(j)) => format!("{}g{}", i, j),
                    (None, Some(j)) => format!("g{}", j),
                }
            }
        };
        let level_name = format!("\"{}\"", line.level_name.replace("\"", "'"));
        let post_name = format!("\"{}\"", cleanup_post_name(&post.title));
        let author = format!("\"{}\"", post.author.replace("\"", "'"));
        let report_line = format!(
            "{},{},{},{},{},{},{}",
            classif, post.score, post.date, author, post_name, level_name, post.url
        );
        report_lines.push(report_line);
    }
    let mut file = File::create("a0f661c5cb36180a3a6aca4bb4d385b2/2puzzles.csv").unwrap();
    file.write_all(HEADER0.as_bytes()).unwrap();
    for report_line in &report_lines {
        file.write_all(report_line.as_bytes()).unwrap();
        file.write_all("\n".as_bytes()).unwrap();
    }
}

pub fn report_ranked(lines: &Vec<Line>) {
    let mut report_lines = vec![];
    for (i, line) in lines.iter().enumerate() {
        let post = &line.post;
        let (max_local, max_global) = match &line.outcome {
            Outcome::ParseFail => continue,
            Outcome::Solver(solver::Outcome::Timeout) => continue,
            Outcome::Solver(solver::Outcome::Unsolvable) => continue,
            Outcome::Solver(solver::Outcome::Solved(findings_vec)) => {
                solver::difficulty_of_findings_vec(&findings_vec)
            }
        };
        // let max_local = max_local as i32;
        // let max_global = max_global as i32;
        let classif = match (max_local, max_global) {
            (None, None) => panic!(),
            (Some(i), None) => format!("{}", i),
            (Some(i), Some(j)) => format!("{}g{}", i, j),
            (None, Some(j)) => format!("g{}", j),
        };
        let level_name = format!("\"{}\"", line.level_name.replace("\"", "'"));
        let post_name = format!("\"{}\"", cleanup_post_name(&post.title));
        let author = format!("\"{}\"", post.author.replace("\"", "'"));
        let report_line = format!(
            "{},{},{},{},{},{},{}",
            classif, post.score, post.date, author, post_name, level_name, post.url
        );
        let key = (
            max_local.map(|i| -(i as i32)).unwrap_or(0),
            max_global.map(|i| -(i as i32)).unwrap_or(0),
            i,
        );
        report_lines.push((key, report_line));
    }
    report_lines.sort();
    let mut file = File::create("a0f661c5cb36180a3a6aca4bb4d385b2/1puzzles_ranked.csv").unwrap();
    file.write_all(HEADER1.as_bytes()).unwrap();
    for (_key, report_line) in &report_lines {
        file.write_all(report_line.as_bytes()).unwrap();
        file.write_all("\n".as_bytes()).unwrap();
    }
}
