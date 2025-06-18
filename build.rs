use std::fs;

use fs_extra::copy_items;
use fs_extra::dir;
use glob::glob;
use minify_html::minify;
use minify_html::Cfg;

fn main() {
    minimize_assets();

    println!("cargo:rerun-if-changed=templates/");
    println!("cargo:rerun-if-changed=assets/");
}

fn minimize_assets() {
    let mut options = dir::CopyOptions::new();
    options.overwrite = true;

    copy_items(&["./assets", "./templates"], "target", &options).unwrap();

    let minify_html_options = Cfg {
        ..Default::default()
    };

    // Don't minify *.hbs minify_html have conflicts with {{expr}}
    for entry in glob("target/assets/**/*.svg")
        .unwrap()
        .chain(glob("target/assets/**/*.css").unwrap())
        .chain(glob("target/assets/**/*.js").unwrap())
    {
        let path = entry.unwrap();
        let data = fs::read(&path).unwrap();
        let minified = minify(&data, &minify_html_options);

        fs::write(path, minified).unwrap();
    }
}
