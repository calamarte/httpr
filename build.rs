use std::env;

use dotenvy::dotenv;

fn main() {
    dotenv().expect("Load env Variables!");

    let api_key = env::var("API_KEY").expect("API_KEY not found!");

    println!("cargo:rustc-env=EMBEDDED_API_KEY={api_key}");
}
