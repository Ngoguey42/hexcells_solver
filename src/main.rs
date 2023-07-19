// https://github.com/oprypin/sixcells
// https://www.redblobgames.com/grids/hexagons/

/*
Custom rules (no crash):
https://www.reddit.com/r/hexcellslevels/comments/ptxsdy/level_pack_uniqueness_1/
https://www.reddit.com/r/hexcellslevels/comments/purwuh/level_pack_uniqueness_2/
https://www.reddit.com/r/hexcellslevels/comments/pmus9n/level_solid_snake_medium/
https://www.reddit.com/r/hexcellslevels/comments/lnff8w/level_hexcells_turns_7_medium/
(18min) https://www.reddit.com/r/hexcellslevels/comments/puptbf/level_sudoku1_medium/
(?) https://www.reddit.com/r/hexcellslevels/comments/poa4rv/level_cube_of_carbon_hard/

Passing fast:
https://www.reddit.com/r/hexcellslevels/comments/gcb9gd/level_the_trial_medium/
https://www.reddit.com/r/hexcellslevels/comments/153f7tb/level_will_you_map_easy/
https://www.reddit.com/r/hexcellslevels/comments/iizzu8/level_hammer_of_judgement_easymedium/
https://www.reddit.com/r/hexcellslevels/comments/jhn6bm/spooky_halloween_puzzle_medium/
https://www.reddit.com/r/hexcellslevels/comments/j40xi1/level_pack_hexagone_mediumhard/
https://www.reddit.com/r/hexcellslevels/comments/6jokbg/level_a_giant_scoop_of_vanilla_3_hard/
https://www.reddit.com/r/hexcellslevels/comments/pj3njy/level_my_first_level_easy/
https://www.reddit.com/r/hexcellslevels/comments/q45wr6/level_not_even_close_easy/

Passing but long:
(6:16) https://www.reddit.com/r/hexcellslevels/comments/it0rag/level_hard_little_hexagon_hard/
(0:41) https://www.reddit.com/r/hexcellslevels/comments/ihwpx6/level_a_giant_scoop_of_vanilla_4_hard/

Solver crash:
(Crash on assert solution count in align) https://www.reddit.com/r/hexcellslevels/comments/pnhjef/level_divided_easy/


Parsing error:
(Bad alignment) https://www.reddit.com/r/hexcellslevels/comments/vrzs8z/level_photo_negative_medium/
(Bad alignment) https://www.reddit.com/r/hexcellslevels/comments/is384q/level_its_hexcells_oclock_medium/
(Bad alignment) https://www.reddit.com/r/hexcellslevels/comments/qxcay1/level_hidden_odds_hard/
https://www.reddit.com/r/hexcellslevels/comments/iumocm/level_tumbling_dice_medium/
https://www.reddit.com/r/hexcellslevels/comments/ig20vp/level_pack_quirky_quints_easyhard/


 */

extern crate regex;

extern crate itertools;

extern crate once_cell;

extern crate serde;

mod constraint;
mod defn;
mod env;
mod misc;
mod multiverse;
mod reddit_post;
mod reporting;
mod solver;

// use regex::Regex;
// use std::env;
use std::error::Error;

// use misc::err;

fn main() -> Result<(), Box<dyn Error>> {
    // let args: Vec<_> = env::args().collect();
    // if args.len() != 2 {
    //     return err("Wrong number of arguments to program");
    // };
    // let url = &args[1];
    // println!("URL: {}", url);

    let mut reporting = vec![];
    let mut env = env::Env::new(2);
    // let mut env = env::Env::new(60 * 20);

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
            let defn = match defn::of_string(&strdefn) {
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
