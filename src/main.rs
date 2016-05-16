extern crate iron;
extern crate persistent;
extern crate urlencoded;

use std::collections::HashMap;
use iron::prelude::*;
use iron::typemap::Key;
use iron::status;
use persistent::Write;
use urlencoded::UrlEncodedQuery;

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

    let mutex = req.get::<Write<EnvVars>>().unwrap();
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
    let mutex = req.get::<Write<EnvVars>>().unwrap();
    let mut vars = mutex.lock().unwrap();

    for (name, value) in processed {
        vars.insert(name.clone(), value);
    }

    Ok(Response::with((status::Ok, "State updated\n")))
}

fn handle_404(_req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::NotFound, "404 Not Found\n")))
}

fn dispatch(req: &mut Request) -> IronResult<Response> {
    let function = match req.url.path[0].as_str() {
        "get" => handle_get,
        "set" => handle_set,
        _ => handle_404,
    };

    function(req)
}

fn main() {
    let mut chain = Chain::new(dispatch);
    chain.link(Write::<EnvVars>::both(HashMap::new()));
    Iron::new(chain).http("localhost:3000").unwrap();
}
