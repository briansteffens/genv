#[macro_use] extern crate hyper;
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

header! { (XSecret, "X-Secret") => [String] }

struct Context<'a> {
    home_dir: &'a str,
    command: &'a str,
    args: Vec<String>,
    config: BTreeMap<String, String>,
}

fn print_usage() {
    println!("Usage:");
    println!("  genv config [server,secret] <VALUE>");
    println!("  genv set <NAME> <VALUE>");
    println!("  genv get <NAME>");
    println!("  genv all");
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
        process::exit(1);
    }

    if !context.config.contains_key("secret") {
        println!("No secret specified. Run genc config secret <SECRET>.");
        process::exit(1);
    }

    let handler = match command.as_ref() {
        "get" => handle_get,
        "set" => handle_set,
        "update" => handle_update,
        _ => {
            print_usage();
            process::exit(1);
        },
    };

    handler(&mut context);
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

fn web_request(context: &Context, fragment: &str) -> String {
    let server = match context.config.get("server") {
        Some(v) => v,
        None => {
            println!("No server found. Run genv config server <SERVER_URL>.");
            process::exit(1);
        },
    };

    let endpoint = format!("{}{}", server, fragment);

    let client = Client::new();
    let mut req = client.get(&endpoint);

    let secret = match context.config.get("secret") {
        Some(v) => v,
        None => {
            println!("No secret found. Run genv config secret <SECRET>.");
            process::exit(1);
        },
    };
    req = req.header(XSecret(secret.to_owned()));

    let mut res = match req.send() {
        Ok(v) => v,
        Err(_) => {
            println!("Error making HTTP request to {}", endpoint);
            process::exit(1);
        },
    };
    assert_eq!(res.status, hyper::Ok);

    let mut body = String::new();
    if res.read_to_string(&mut body).is_err() {
        println!("Error reading HTTP response body");
        process::exit(1);
    }
    body
}

fn handle_get(context: &mut Context) {
    if context.args.len() != 1 {
        print_usage();
        process::exit(1);
    }

    let fragment = format!("get/{}", context.args[0]);
    let res = web_request(&context, &fragment);
    println!("{}", res);
}

fn handle_set(context: &mut Context) {
    if context.args.len() != 2 {
        print_usage();
        process::exit(1);
    }

    let fragment = format!("set?{}={}", context.args[0], context.args[1]);
    web_request(&context, &fragment);
}

fn handle_update(context: &mut Context) {
    if context.args.len() != 0 {
        print_usage();
        process::exit(1);
    }

    // HTTP request for all genv variables
    let body = web_request(&context, "all");

    let all: BTreeMap<String, String> = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(_) => {
            println!("Unable to parse JSON response from server");
            process::exit(1)
        },
    };

    // Write to .genv
    let mut output = String::new();

    for (key, value) in all {
        output.push_str(&format!("export {}=\"{}\"\n", &key, &value));
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
