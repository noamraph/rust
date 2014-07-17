// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

struct Point {
    x: int,
}

fn main() {
    let p = Point { x: 3 }; //ignore-tidy-linelength //~ WARNING Use '=' instead of ':' in struct expressions. You can use the rust-update-structs tool to automate the replacement process.
    let Point { x: x0 } = p; //ignore-tidy-linelength //~ WARNING Use '=' instead of ':' in struct expressions. You can use the rust-update-structs tool to automate the replacement process.
    let () = p; //~ ERROR mismatched types: expected `Point` but found `()`
}
