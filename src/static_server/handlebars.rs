use core::str;

use handlebars::{
    Context, Handlebars, Helper, HelperResult, Output, RenderContext, RenderErrorReason,
};
use once_cell::sync::Lazy;
use rust_embed::RustEmbed;

use super::INTERNAL_ROOT;

pub const DIRECTORY_TEMPLATE: &str = "directory";
pub const NOT_FOUND_TEMPLATE: &str = "not_found";

const MIME_FALLBACK_PATH: &str = "icons/file.svg";

#[derive(RustEmbed)]
#[folder = "target/assets/"]
pub struct Assets;

pub static HBS: Lazy<Handlebars<'static>> = Lazy::new(|| {
    let mut hbs = Handlebars::new();
    hbs.register_template_string(
        DIRECTORY_TEMPLATE,
        include_str!("../../target/templates/directory.hbs"),
    )
    .unwrap();

    hbs.register_template_string(
        NOT_FOUND_TEMPLATE,
        include_str!("../../target/templates/not_found.hbs"),
    )
    .unwrap();

    // internal path
    hbs.register_helper(
        "internal_path",
        Box::new(
            |_: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let _ = out.write(INTERNAL_ROOT);
                Ok(())
            },
        ),
    );

    // assets inject
    hbs.register_helper(
        "asset",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param = h
                    .param(0)
                    .ok_or(RenderErrorReason::ParamNotFoundForIndex("asset", 0))?;

                if let Some(asset) = Assets::get(
                    param
                        .value()
                        .as_str()
                        .ok_or(RenderErrorReason::InvalidParamType("Invalid"))?,
                ) {
                    let data = str::from_utf8(&asset.data).unwrap();
                    out.write(data)?;
                }

                Ok(())
            },
        ),
    );

    // Get icon by mime
    hbs.register_helper(
        "icon_by_mime",
        Box::new(
            |h: &Helper,
             _: &Handlebars,
             _: &Context,
             _: &mut RenderContext,
             out: &mut dyn Output|
             -> HelperResult {
                let param = h
                    .param(0)
                    .ok_or(RenderErrorReason::ParamNotFoundForIndex("asset", 0))?;

                let value = param.value().as_str();

                if value.is_none() {
                    let data = Assets::get(MIME_FALLBACK_PATH).unwrap().data;
                    let _ = out.write(str::from_utf8(&data).unwrap());
                    return Ok(());
                }

                let path = format!("icons/by_mime/{}.svg", value.unwrap());

                if let Some(icon) = Assets::get(&path) {
                    let data = str::from_utf8(&icon.data).unwrap();
                    out.write(data)?;
                } else {
                    let data = Assets::get(MIME_FALLBACK_PATH).unwrap().data;
                    let _ = out.write(str::from_utf8(&data).unwrap());
                    return Ok(());
                }

                Ok(())
            },
        ),
    );

    hbs
});
