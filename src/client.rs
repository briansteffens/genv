extern crate hyper;
extern crate serde;
extern crate serde_json;

use std::env;
use std::process;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
//use hyper::*;

fn print_usage() {
    println!("Usage:");
    println!("  genv set <NAME> <VALUE>");
    println!("  genv get <NAME>");
    println!("  genv all");
}

fn read_config() -> BTreeMap<String, String> {
    let mut config_file = match File::open("/home/brian/.genv") {
        Ok(v) => v,
        Err(_) => return BTreeMap::new(),
    };

    let mut config = String::new();
    if config_file.read_to_string(&mut config).is_err() {
        return BTreeMap::new();
    }

    return match serde_json::from_str(&config) {
        Ok(v) => v,
        Err(_) => return BTreeMap::new(),
    };
}

fn save_config(config: &mut BTreeMap<String, String>) {
    let mut file = match File::create("/home/brian/.genv") {
        Ok(v) => v,
        Err(_) => panic!("Error opening file for writing"),
    };

    let serialized = match serde_json::to_string(&config) {
        Ok(v) => v,
        Err(_) => panic!("Failed to serialize JSON"),
    };

    if file.write_all(serialized.as_bytes()).is_err() {
        panic!("Failed to write to file");
    }
}

fn main() {
    let mut argv = env::args();
    argv.next();

    let command = match argv.next() {
        Some(v) => v,
        None => return print_usage(),
    };

    let mut args: Vec<String> = argv.collect();

    let mut config = read_config();

    if command == "config" {
        handle_config(&mut config, &mut args);
        save_config(&mut config);
        return;
    }

    let handler = match command.as_ref() {
        "get" => handle_get,
        "set" => handle_set,
        "update" => handle_update,
        _ => return print_usage(),
    };

    if !handler(&config, &mut args) {
        print_usage();
    }
}

fn handle_config(config: &mut BTreeMap<String, String>, args: &mut Vec<String>)
        {
    if args.len() != 2 {
        print_usage();
        process::exit(1);
    }

    if args[0] != "server" && args[0] != "secret" {
        println!("Valid config values are server and secret");
        process::exit(1);
    }

    config.insert(args[0].clone(), args[1].clone());
}

fn handle_get(config: &BTreeMap<String, String>, args: &mut Vec<String>)
        -> bool {
    if args.len() != 1 {
        return false;
    }

    println!("get {}", args[0]);

    true
}

fn handle_set(config: &BTreeMap<String, String>, args: &mut Vec<String>)
        -> bool {
    if args.len() != 2 {
        return false;
    }

    println!("set {} => {}", args[0], args[1]);

    true
}

fn handle_update(config: &BTreeMap<String, String>, args: &mut Vec<String>)
        -> bool {
    if args.len() != 0 {
        return false;
    }

    true
}
