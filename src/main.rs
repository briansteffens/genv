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

fn join_path(path: &Vec<String>) -> String {
    let mut ret = path.join("/");

    // Remove trailing slash
    if ret.ends_with("/") {
        ret.pop();
    }

    ret
}

fn bad_request(content: &str) -> IronResult<Response> {
    Ok(Response::with((status::BadRequest, content)))
}

fn handle_get(req: &mut Request) -> IronResult<Response> {
    let name = {
        let query = match req.get_ref::<UrlEncodedQuery>() {
            Ok(hashmap) => hashmap,
            Err(_e) => return bad_request("Querystring parse error\n"),
        };

        if query.len() != 1 {
            return bad_request("Invalid number of querystring parameters\n");
        }

        let name_vals = match query.get("name") {
            Some(v) => v,
            None    => return bad_request("Requested parameter not found\n"),
        };

        if name_vals.len() != 1 {
            return bad_request("Invalid number of values\n");
        }

        name_vals[0].clone()
    };

    let mutex = req.get::<Write<EnvVars>>().unwrap();
    let vars = mutex.lock().unwrap();

    let value = match vars.get(name.as_str()) {
        Some(v) => v,
        None    => return Ok(Response::with((status::BadRequest,
                             "No value found by that name\n"))),
    };

    Ok(Response::with((status::Ok, value.as_str())))
}

fn handle_404(_req: &mut Request) -> IronResult<Response> {
    Ok(Response::with((status::NotFound, "404 Not Found\n")))
}

fn dispatch(req: &mut Request) -> IronResult<Response> {
    let path = join_path(&req.url.path);

    println!("REQ: {}", path);

    let function = match path.as_str() {
        "get" => handle_get,
        _ => handle_404,
    };

    function(req)
}

fn main() {
    let mut chain = Chain::new(dispatch);
    chain.link(Write::<EnvVars>::both(HashMap::new()));
    Iron::new(chain).http("localhost:3000").unwrap();
}
