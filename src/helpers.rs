handlebars_helper!(base64_encode: |s: str| base64::encode(s));
handlebars_helper!(base64_decode: |s: str| {
    let buf = base64::decode(s).map_err(|e| handlebars::RenderError::from_error("base64 decode failure", e))?;
    String::from_utf8_lossy(buf.as_slice()).into_owned()
});
