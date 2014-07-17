// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(phase)]
extern crate regex;
#[phase(plugin)] extern crate regex_macros;
extern crate getopts;

use std::os;
use std::io::{BufferedReader, BufferedWriter, File, Open, Write, fs};
use std::collections::{HashMap, HashSet};
use std::collections::TreeSet;
use std::iter::AdditiveIterator;
use getopts::{optflag,getopts,OptGroup};

/// Replace ':' in the given string in the given positions by '='.
///
/// s: the string to be processed
/// lcs: a TreeSet of (line, column) tuples
/// path: used for error reporting
///
/// Return a vector of byte offsets into the string, and the number of failures.
/// We use a TreeSet because iterating over it is sorted.
fn replace_colons(s: &str, lcs: &TreeSet<(int, int)>, path: &Path) -> (String, int) {
    let mut r = String::with_capacity(s.len());
    let mut line = 1i;
    let mut col = 0i;
    let mut chars = s.chars();
    let mut last_was_colon = false;
    let mut n_failures: int = 0;

    for &lc in lcs.iter() {
        let mut found = false;
        for ch in chars {
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            if (line, col) == lc {
                if ch == ':' {
                    r.push_char('=');
                    last_was_colon = true;
                } else {
                    n_failures += 1;
                    println!("{}:{}:{}: expected ':', found '{}'.",
                             path.as_str().unwrap(), line, col, ch);
                    r.push_char(ch);
                }
                found = true;
                break;
            } else {
                if last_was_colon {
                    // Remove the ' ' after the colon
                    last_was_colon = false;
                    if ch != ' ' {
                        r.push_char(ch);
                    }
                } else {
                    r.push_char(ch);
                }
            }
        }
        if !found {
            n_failures += 1;
            let (line, col) = lc;
            println!("{} doesn't have line {} column {}",
                     path.as_str().unwrap(), line, col);
        }
    }
    for ch in chars {
        if last_was_colon {
            // Remove the ' ' after the colon
            last_was_colon = false;
            if ch != ' ' {
                r.push_char(ch);
            }
        } else {
            r.push_char(ch);
        }
    }
    return (r, n_failures);
}

fn print_usage(program: &str, _opts: &[OptGroup]) {
    println!("\
Usage: {} [--dry-run] path-prefix build-output-filename

Search build-output-filename for struct expression warnings, and update all the
files that are prefixed by path-prefix.

--dry-run   Just check that all expected ':' are found, without replacing.
-h --help   Print this help message.
", program);
}

fn main() {
    let args: Vec<String> = os::args();

    let program = args.get(0).clone();

    let opts = [
        optflag("", "dry-run",
                "Just check that all expected ':' are found, without replacing."),
        optflag("h", "help", "Print this help message.")
    ];
    let matches = getopts(args.tail(), opts).unwrap();
    if matches.opt_present("h") || matches.free.len() != 2 {
        print_usage(program.as_slice(), opts);
        return;
    }
    let is_dry_run: bool = matches.opt_present("dry-run");
    let prefix = Path::new(matches.free.get(0).as_slice());
    let build_output_path = Path::new(matches.free.get(1).as_slice());


    let re = regex!(r"(/[^ \n]+):(\d+):(\d+): (\d+):(\d+) warning: Use '=' instead of ':'");
    let text = BufferedReader::new(File::open(&build_output_path)).read_to_str().unwrap();

    let mut places: HashMap<Path, TreeSet<(int, int)>> = HashMap::new();
    let mut ignored_paths: HashSet<Path> = HashSet::new();
    let mut unlocated_paths: HashSet<Path> = HashSet::new();

    for caps in re.captures_iter(text.as_slice()) {
        let path = Path::new(caps.at(1));
        let l0 = from_str::<int>(caps.at(2)).unwrap();
        let c0 = from_str::<int>(caps.at(3)).unwrap();
        let l1 = from_str::<int>(caps.at(4)).unwrap();
        let c1 = from_str::<int>(caps.at(5)).unwrap();
        if !prefix.is_ancestor_of(&path) {
            ignored_paths.insert(path);
        } else {
            if l0 == 1 && c0 == 1 && l1 == 1 && c1 == 1 {
                unlocated_paths.insert(path);
            } else {
                assert!(l1 == l0 && c1 == c0 + 1);
                places.find_or_insert_with(path, |_k| TreeSet::new()).insert((l0, c0));
            }
        }
    }
    let n_colons = places.values().map(|s| s.len()).sum();
    println!("Found {} colons in {} files ({} files not in {}).",
             n_colons, places.len(), ignored_paths.len(), prefix.as_str().unwrap());
    for path in unlocated_paths.iter() {
        println!("Unlocated colons in {}", path.as_str().unwrap());
    }

    println!("Validating...");
    let mut total_n_failures = 0;
    for (path, lcs) in places.iter() {
        let s = BufferedReader::new(File::open(path)).read_to_str().unwrap();
        let (_s2, n_failures) = replace_colons(s.as_slice(), lcs, path);
        total_n_failures += n_failures;
    }
    if total_n_failures == 0 {
        println!("Done.");
    } else {
        println!("There were {} failed matches. Will not replace anything.",
                 total_n_failures);
        os::set_exit_status(1);
        return;
    }

    if is_dry_run {
        return;
    }

    println!("Replacing...")
    for (path, lcs) in places.iter() {
        let s = BufferedReader::new(File::open(path)).read_to_str().unwrap();
        let (s_new, n_failures) = replace_colons(s.as_slice(), lcs, path);
        assert!(n_failures == 0);
        let tmp_path = Path::new(String::from_str(path.as_str().unwrap())
                                 .append(".struct-tmp"));
        let backup_path = Path::new(String::from_str(path.as_str().unwrap())
                                    .append(".struct-backup"));
        {
            let mut writer = BufferedWriter::new(File::open_mode(&tmp_path, Open, Write));
            writer.write_str(s_new.as_slice()).unwrap();
            writer.flush().unwrap();
        }
        fs::rename(path, &backup_path).unwrap();
        fs::rename(&tmp_path, path).unwrap();
    }
    println!("Done.");
}

