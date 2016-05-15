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

fn get_querystring_value(req: &mut Request, name: &str)
        -> Result<String, &'static str> {
    let query = match req.get_ref::<UrlEncodedQuery>() {
        Ok(hashmap) => hashmap,
        Err(_e) => return Err("Querystring parse error"),
    };

    if query.len() != 1 {
        return Err("Invalid number of querystring parameters");
    }

    let name_vals = match query.get(name) {
        Some(v) => v,
        None    => return Err("Requested parameter not found"),
    };

    if name_vals.len() != 1 {
        return Err("Invalid number of values");
    }

    Ok(name_vals[0].clone())
}

fn handle_get(req: &mut Request) -> IronResult<Response> {
    fn parameter_error() -> IronResult<Response> {
        Ok(Response::with((status::BadRequest,
                "Expected 1 parameter: 'name'\n")))
    }

    let name = match get_querystring_value(req, "name") {
        Ok(v)   => v,
        Err(_e) => return parameter_error(),
    };

    let mutex = req.get::<Write<EnvVars>>().unwrap();
    let mut vars = mutex.lock().unwrap();

    let value = match vars.get(name.as_str()) {
        Some(v) => v,
        None    => return Ok(Response::with((status::BadRequest,
                             "No value found by that name"))),
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
