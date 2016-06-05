extern crate iron;
extern crate persistent;
extern crate urlencoded;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate hyper;

use std::collections::{HashMap, BTreeMap};
use std::process;
use std::fs::File;
use std::io::prelude::*;
use std::ops::Deref;
use iron::prelude::*;
use iron::typemap::Key;
use iron::status;
use persistent::Read as PersistentRead;
use persistent::Write as PersistentWrite;
use urlencoded::UrlEncodedQuery;

static CONFIG_FN: &'static str = "/etc/genv/config.json";
static STATE_FN: &'static str = "/etc/genv/state.json";

header! { (XSecret, "X-Secret") => [String] }

#[derive(Copy, Clone)]
pub struct Config;

impl Key for Config {
    type Value = HashMap<String, String>;
}

#[derive(Copy, Clone)]
pub struct EnvVars;

impl Key for EnvVars {
    type Value = HashMap<String, String>;
}

fn bad_request(content: &str) -> IronResult<Response> {
    Ok(Response::with((status::BadRequest, content)))
}

fn handle_get(req: &mut Request) -> IronResult<Response> {
    if req.url.path.len() != 2 {
        return Ok(Response::with((status::BadRequest,
                  "Expected one and only one value to get by name")));
    }

    let name = req.url.path[1].clone();

    let mutex = req.get::<PersistentWrite<EnvVars>>().unwrap();
    let vars = mutex.lock().unwrap();

    let value = match vars.get(name.as_str()) {
        Some(v) => v,
        None    => return Ok(Response::with((status::BadRequest,
                             "No value found by that name\n"))),
    };

    Ok(Response::with((status::Ok, value.as_str())))
}

fn handle_set(req: &mut Request) -> IronResult<Response> {
    // Convert HashMap<String, Vec<String>> to HashMap<String, String>.
    // Doing this in a separate pass before applying the values to the state
    // in order to catch any validation errors before making changes to improve
    // atomicity.
    let mut processed = HashMap::new();

    {
        let query = match req.get_ref::<UrlEncodedQuery>() {
            Ok(hashmap) => hashmap,
            Err(_e) => return bad_request("Querystring parse error\n"),
        };

        if query.len() == 0 {
            return bad_request("No values supplied to set\n");
        }

        for (name, value) in query {
            if value.len() != 1 {
                return bad_request("Expected 1 and only 1 value per name\n");
            }

            processed.insert(name.clone(), value[0].clone());
        }
    }

    // Apply changes
    let mutex = req.get::<PersistentWrite<EnvVars>>().unwrap();
    let mut vars = mutex.lock().unwrap();

    for (name, value) in processed {
        vars.insert(name.clone(), value);
    }

    save_state(vars.deref());

    Ok(Response::with((status::Ok, "State updated\n")))
}

fn handle_all(req: &mut Request) -> IronResult<Response> {
    let mut var_map: BTreeMap<String, String> = BTreeMap::new();

    let mutex = req.get::<PersistentWrite<EnvVars>>().unwrap();
    let vars = mutex.lock().unwrap();

    for (name, value) in &*vars {
        var_map.insert(name.clone(), value.clone());
    }

    match serde_json::to_string(&var_map) {
        Ok(v)   => Ok(Response::with((status::Ok, v))),
        Err(_e) => Ok(Response::with((status::InternalServerError,
                      "Failed to serialize JSON response"))),
    }
}

fn handle_404(_req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::NotFound, "404 Not Found\n")))
}

fn dispatch(req: &mut Request) -> IronResult<Response> {
    let config = req.get::<PersistentRead<Config>>().unwrap().clone();

    {
        // Read X-Secret header from request
        let req_secret = match req.headers.get::<XSecret>() {
            Some(v) => v,
            None => {
                return Ok(Response::with((status::Unauthorized,
                          "401 Unauthorized missing X-Secret header\n")));
            },
        };

        let secret = match config.get("secret") {
            Some(v) => v,
            None => {
                return Ok(Response::with((status::InternalServerError,
                          "500 Internal Server Error no secret in config\n")));
            },
        };

        // Check X-Secret header against secret in config
        if secret != &req_secret.to_string() {
            return Ok(Response::with((status::Unauthorized,
                      "401 Unauthorized incorrect X-Secret header value\n")));
        }
    }

    // Route request to handler
    let function = match req.url.path[0].as_str() {
        "get" => handle_get,
        "set" => handle_set,
        "all" => handle_all,
        _ => handle_404,
    };

    function(req)
}

fn main() {
    let mut chain = Chain::new(dispatch);
    chain.link(PersistentRead::<Config>::both(read_config()));
    chain.link(PersistentWrite::<EnvVars>::both(read_state()));
    Iron::new(chain).http("localhost:3000").unwrap();
}

fn read_config() -> HashMap<String, String> {
    let mut config_file = match File::open(CONFIG_FN) {
        Ok(v) => v,
        Err(_) => {
            println!("Unable to open config file /etc/genv-server.conf");
            process::exit(1);
        },
    };

    let mut config = String::new();
    if config_file.read_to_string(&mut config).is_err() {
        println!("Unable to read from config file");
        process::exit(1);
    }

    let ret: HashMap<String, String> = match serde_json::from_str(&config) {
        Ok(v) => v,
        Err(_) => {
            println!("Unable to parse config file");
            process::exit(1);
        },
    };

    if !ret.contains_key("secret") {
        println!("Config file missing key 'secret'");
        process::exit(1);
    }

    ret
}

fn read_state() -> HashMap<String, String> {
    let mut file = match File::open(STATE_FN) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };

    let mut state = String::new();
    if file.read_to_string(&mut state).is_err() {
        return HashMap::new();
    }

    return match serde_json::from_str(&state) {
        Ok(v) => v,
        Err(_) => return HashMap::new(),
    };
}

fn save_state(vars: &HashMap<String, String>) {
    let mut file = match File::create(STATE_FN) {
        Ok(v) => v,
        Err(_) => {
            println!("Unable to open state file for writing: {}", STATE_FN);
            process::exit(1);
        },
    };

    let serialized = match serde_json::to_string(&vars) {
        Ok(v) => v,
        Err(_) => {
            println!("Failed to serialize state as JSON");
            process::exit(1);
        },
    };

    if file.write_all(serialized.as_bytes()).is_err() {
        println!("Failed to write to state file: {}", STATE_FN);
        process::exit(1);
    }
}
