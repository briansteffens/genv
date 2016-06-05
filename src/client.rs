extern crate hyper;
extern crate serde;
extern crate serde_json;

use std::env;
use std::process;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use hyper::Client;

static CONFIG_FN: &'static str = ".genv.conf";
static GENV_FN: &'static str = ".genv";

struct Context<'a> {
    home_dir: &'a str,
    command: &'a str,
    args: Vec<String>,
    config: BTreeMap<String, String>,
}

fn print_usage() {
    println!("Usage:");
    println!("  genv set <NAME> <VALUE>");
    println!("  genv get <NAME>");
    println!("  genv all");
}

fn read_config(context: &mut Context) {
    let config_fn = format!("{}/{}", context.home_dir, CONFIG_FN);
    let mut config_file = match File::open(config_fn) {
        Ok(v) => v,
        Err(_) => return,
    };

    let mut config = String::new();
    if config_file.read_to_string(&mut config).is_err() {
        return;
    }

    context.config = match serde_json::from_str(&config) {
        Ok(v) => v,
        Err(_) => return,
    };
}

fn save_config(context: &mut Context) {
    let config_fn = format!("{}/{}", context.home_dir, CONFIG_FN);
    let mut file = match File::create(config_fn) {
        Ok(v) => v,
        Err(_) => {
            println!("Error opening config file for writing");
            process::exit(1);
        },
    };

    let serialized = match serde_json::to_string(&context.config) {
        Ok(v) => v,
        Err(_) => {
            println!("Failed to serialize JSON");
            process::exit(1);
        },
    };

    if file.write_all(serialized.as_bytes()).is_err() {
        println!("Failed to write to config file");
        process::exit(1);
    }
}

fn main() {
    // TODO: Figure out a better way to do these two statements
    let home_dir_path = match env::home_dir() {
        Some(v) => v,
        None => {
            println!("Unable to figure out the current user's home directory");
            process::exit(1);
        },
    };

    let home_dir = match home_dir_path.to_str() {
        Some(v) => v,
        None => {
            println!("Unable to convert PathBuf to string??");
            process::exit(1);
        },
    };

    let mut argv = env::args();
    argv.next();

    let command = match argv.next() {
        Some(v) => v,
        None => return print_usage(),
    };

    let mut context = Context {
        home_dir: home_dir,
        command: &command,
        args: argv.collect(),
        config: BTreeMap::new(),
    };

    read_config(&mut context);

    if context.command == "config" {
        handle_config(&mut context);
        save_config(&mut context);
        return;
    }

    if !context.config.contains_key("server") {
        println!("No server specified. Run genv config server <SERVER_URL>.");
        return;
    }

    if !context.config.contains_key("secret") {
        println!("No secret specified. Run genc config secret <SECRET>.");
        return;
    }

    let handler = match command.as_ref() {
        "get" => handle_get,
        "set" => handle_set,
        "update" => handle_update,
        _ => return print_usage(),
    };

    handler(&mut context);
}

fn handle_config(context: &mut Context) {
    if context.args.len() != 2 {
        print_usage();
        process::exit(1);
    }

    if context.args[0] != "server" && context.args[0] != "secret" {
        println!("Valid config values are server and secret");
        process::exit(1);
    }

    context.config.insert(context.args[0].clone(), context.args[1].clone());
}

fn handle_get(context: &mut Context) {
    if context.args.len() != 1 {
        print_usage();
        process::exit(1);
    }

    let endpoint = format!("{}get/{}", context.config.get("server").unwrap(),
            context.args[0]);

    let client = Client::new();
    let mut res = client.get(&endpoint).send().unwrap();
    assert_eq!(res.status, hyper::Ok);
    let mut s = String::new();
    res.read_to_string(&mut s).unwrap();
    println!("{}", s);
}

fn handle_set(context: &mut Context) {
    if context.args.len() != 2 {
        print_usage();
        process::exit(1);
    }

    let endpoint = format!("{}set?{}={}",
            context.config.get("server").unwrap(),
            context.args[0], context.args[1]);

    let client = Client::new();
    let res = client.get(&endpoint).send().unwrap();
    assert_eq!(res.status, hyper::Ok);
}

fn handle_update(context: &mut Context) {
    if context.args.len() != 0 {
        print_usage();
        process::exit(1);
    }

    // HTTP request for all genv variables
    let endpoint = format!("{}all", context.config.get("server").unwrap());

    let client = Client::new();
    let mut res = client.get(&endpoint).send().unwrap();
    assert_eq!(res.status, hyper::Ok);

    let mut s = String::new();
    res.read_to_string(&mut s).unwrap();

    let all: BTreeMap<String, String> = match serde_json::from_str(&s) {
        Ok(v) => v,
        Err(_) => {
            println!("Unable to parse JSON response from server");
            process::exit(1)
        },
    };

    // Write to .genv
    let mut output = String::new();

    for (key, value) in all {
        output.push_str(&key);
        output.push_str("=\"");
        output.push_str(&value);
        output.push_str("\"\n");
    }

    let genv_fn = format!("{}/{}", context.home_dir, GENV_FN);
    let mut file = match File::create(genv_fn) {
        Ok(v) => v,
        Err(_) => {
            println!("Error opening .genv for writing");
            process::exit(1);
        },
    };

    if file.write_all(output.as_bytes()).is_err() {
        println!("Error writing to .genv");
        process::exit(1);
    }

    // Read .bashrc
    let bashrc_fn = format!("{}/.bashrc", context.home_dir);
    let mut bashrc = String::new();

    {
        let mut bashrc_file = match File::open(&bashrc_fn) {
            Ok(v) => v,
            Err(_) => {
                println!("Error opening .bashrc");
                process::exit(1);
            },
        };

        if bashrc_file.read_to_string(&mut bashrc).is_err() {
            println!("Error reading from .bashrc");
            process::exit(1);
        }
    }

    // Check for .genv inclusion in .bashrc
    if (&bashrc).contains("source ~/.genv") {
        return;
    }

    // Add .genv to .bashrc
    bashrc.push_str("\nsource ~/.genv\n");

    // Write changes to .bashrc
    let mut bashrc_file = match File::create(&bashrc_fn) {
        Ok(v) => v,
        Err(_) => {
            println!("Error opening .bashrc for writing");
            process::exit(1);
        },
    };

    if bashrc_file.write_all(bashrc.as_bytes()).is_err() {
        println!("Error writing to .bashrc");
        process::exit(1);
    }
}
