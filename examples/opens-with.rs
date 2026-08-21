//! Ask the platform what would open a payload of a given name.
//!
//! The card's answer without a window around it, so the question can be tried
//! on a machine before there is a container to try it with. Slice 11 has macOS
//! and Windows to implement and this is where they get exercised.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

fn main() {
    let names: Vec<String> = std::env::args().skip(1).collect();
    if names.is_empty() {
        eprintln!("usage: opens-with <payload-name>...");
        return;
    }
    for name in names {
        match slipcase_desktop::opens_with::opens_with(&name) {
            Some(application) => println!("{name}: {application}"),
            None => println!("{name}: (the platform did not answer)"),
        }
    }
}
