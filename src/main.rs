extern crate itertools;
extern crate once_cell;
extern crate regex;
extern crate serde;

mod constraint;
mod defn;
mod env;
mod misc;
mod multiverse;
mod reddit_post;
mod reporting;
mod solver;

use std::env::args;
use std::error::Error;
use std::io;

fn main_stdin() -> Result<(), Box<dyn Error>> {
    let mut strdefn = String::new();
    let stdin = io::stdin();
    for _ in 0..38 {
        let mut line = String::new();
        stdin.read_line(&mut line)?;
        strdefn.push_str(&line);
    }
    let defn = defn::of_string(&strdefn)?;
    let mut env = env::Env::new(3600 * 24 * 30);
    let outcome = solver::solve(&mut env, &defn, false);
    println!("{}", outcome);
    println!("{:?}", outcome);
    Ok(())
}

fn main_reddit_posts() -> Result<(), Box<dyn Error>> {
    let mut reporting = vec![];
    let mut env = env::Env::new(60 * 20);

    let reddit_posts = reddit_post::list_levels("./reddit_posts.json")?;
    for post in reddit_posts {
        println!("> {:?}", post);
        let strdefns = reddit_post::strdefns_of_post(&post, "./cache_reqwest")?;
        println!("  {} puzzles(s)", strdefns.len());
        for (idx_in_post, strdefn) in strdefns.iter().enumerate() {
            let idx_in_post = idx_in_post as u32;
            let level_name = strdefn
                .split('\n')
                .nth(1)
                .unwrap()
                .replace("&#39;", "'")
                .trim()
                .to_string();
            let defn = match defn::of_string(strdefn) {
                Err(err) => {
                    reporting.push(reporting::Line {
                        post: post.clone(),
                        idx_in_post,
                        level_name,
                        outcome: reporting::Outcome::ParseFail,
                    });
                    println!("  Skip because {:?}", err);
                    continue;
                }
                Ok(defn) => defn,
            };
            let outcome = misc::with_cache(
                &strdefn.trim(),
                || Ok(solver::solve(&mut env, &defn, false)),
                "./cache_solver",
            )?;
            println!("  Outcome: {}", outcome);
            reporting.push(reporting::Line {
                post: post.clone(),
                idx_in_post,
                level_name,
                outcome: reporting::Outcome::Solver(outcome),
            });
        }
    }
    reporting::report_ranked(&reporting);
    reporting::report_all(&reporting);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = args().collect();
    if args.len() != 2 {
        Err("Wrong number of arguments to program".into())
    } else if args[1] == "reddit-posts" {
        main_reddit_posts()
    } else if args[1] == "-" {
        main_stdin()
    } else {
        Err("Wrong argument to program".into())
    }
}
