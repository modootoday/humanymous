//! Bounded W3C command transport, artifact identity and measured keyboard actions.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{net::IpAddr, path::Path, time::Instant};
use url::Url;
const ELEMENT: &str = "element-6066-11e4-a52e-4f735466cecf";
pub(super) fn local_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "invalid local pilot URL")?;
    let loopback = url.host_str().is_some_and(|host| {
        host.trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
    });
    if url.scheme() != "http"
        || !loopback
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("pilot requires credential-free literal loopback HTTP URLs".into());
    }
    Ok(url)
}

pub(super) fn segment(value: &Value, key: &str) -> Result<String, String> {
    let value = value[key].as_str().ok_or("missing WebDriver identity")?;
    if value.is_empty()
        || value.len() > 256
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err("invalid WebDriver identity".into());
    }
    Ok(value.to_owned())
}

pub(super) fn usable_css(property: &str, value: &Value) -> bool {
    let Some(value) = value.as_str().filter(|value| !value.is_empty()) else {
        return false;
    };
    match property {
        "display" => value != "none",
        "visibility" => value == "visible",
        "opacity" => value
            .parse::<f64>()
            .is_ok_and(|n| n.is_finite() && n > 0.0 && n <= 1.0),
        _ => false,
    }
}

pub(super) struct Driver {
    pub(super) endpoint: Url,
    pub(super) agent: ureq::Agent,
    pub(super) deadline: Instant,
}
impl Driver {
    pub(super) fn command(&self, method: &str, path: &str, body: Value) -> Result<Value, String> {
        if method != "DELETE" && Instant::now() >= self.deadline {
            return Err("browser pilot time budget exhausted".into());
        }
        let url = self
            .endpoint
            .join(path)
            .map_err(|_| "invalid driver path")?;
        let response = match method {
            "GET" => self.agent.get(url.as_str()).call(),
            "POST" => self
                .agent
                .post(url.as_str())
                .header("Content-Type", "application/json")
                .send(body.to_string()),
            "DELETE" => self.agent.delete(url.as_str()).call(),
            _ => return Err("unsupported WebDriver method".into()),
        }
        .map_err(|_| "WebDriver HTTP command failed")?;
        if response.status().as_u16() != 200 {
            return Err("unexpected WebDriver HTTP status".into());
        }
        let text = response
            .into_body()
            .into_with_config()
            .limit(1_048_576)
            .read_to_string()
            .map_err(|_| "WebDriver response read failed")?;
        let response: Value = serde_json::from_str(&text).map_err(|_| "invalid WebDriver JSON")?;
        let value = response.get("value").ok_or("missing WebDriver value")?;
        if value.get("error").is_some() {
            return Err("WebDriver protocol error".into());
        }
        Ok(value.clone())
    }
}

pub(super) fn served_artifacts(
    driver: &Driver,
    base: &Url,
    site: &Path,
) -> Result<(usize, String), String> {
    let mut digest = Sha256::new();
    let mut count = 0;
    for path in crate::files::walk(site, false)? {
        if Instant::now() >= driver.deadline {
            return Err("browser artifact verification time budget exhausted".into());
        }
        let relative = crate::files::relative(site, &path);
        let url = base
            .join(&relative)
            .map_err(|_| "invalid served artifact URL")?;
        if url.origin() != base.origin() || !url.path().starts_with(base.path()) {
            return Err("served artifact URL escaped its prefix".into());
        }
        if std::fs::metadata(&path)
            .map_err(|_| "cannot inspect built artifact")?
            .len()
            > 8_388_608
        {
            return Err("built artifact exceeds pilot byte bound".into());
        }
        let expected = std::fs::read(&path).map_err(|_| "cannot read built artifact")?;
        let response = driver
            .agent
            .get(url.as_str())
            .header("Cache-Control", "no-cache")
            .call()
            .map_err(|_| "served artifact request failed")?;
        if response.status().as_u16() != 200 {
            return Err("served artifact returned non-200 status".into());
        }
        let bytes = response
            .into_body()
            .into_with_config()
            .limit(8_388_608)
            .read_to_vec()
            .map_err(|_| "served artifact exceeds bound or cannot be read")?;
        if bytes != expected {
            return Err("served artifact bytes differ from checked output".into());
        }
        digest.update((relative.len() as u64).to_be_bytes());
        digest.update(relative.as_bytes());
        digest.update(Sha256::digest(&bytes));
        count += 1;
    }
    Ok((count, hex::encode(digest.finalize())))
}

fn keyboard(driver: &Driver, session: &str, key: &str) -> Result<(), String> {
    driver.command(
        "POST",
        &format!("session/{session}/actions"),
        json!({
            "actions":[{"type":"key","id":"pages-keyboard","actions":[
                {"type":"keyDown","value":key},{"type":"keyUp","value":key}
            ]}]
        }),
    )?;
    Ok(())
}

pub(super) fn page(
    driver: &Driver,
    session: &str,
    url: &Url,
    width: u32,
    height: u32,
    observed: &mut Value,
) -> Result<(), String> {
    let prefix = format!("session/{session}");
    let window = driver.command(
        "POST",
        &format!("{prefix}/window/rect"),
        json!({"width":width,"height":height}),
    )?;
    observed["window"] = window.clone();
    if window["width"].as_u64() != Some(u64::from(width))
        || window["height"].as_u64() != Some(u64::from(height))
    {
        return Err("browser did not apply the requested window size".into());
    }
    driver.command(
        "POST",
        &format!("{prefix}/url"),
        json!({"url":url.as_str()}),
    )?;
    let actual = driver.command("GET", &format!("{prefix}/url"), Value::Null)?;
    observed["url"] = actual.clone();
    if actual.as_str() != Some(url.as_str()) {
        return Err("browser navigation redirected or changed the page URL".into());
    }
    let title = driver.command("GET", &format!("{prefix}/title"), Value::Null)?;
    observed["title"] = title.clone();
    if title.as_str().is_none_or(|title| title.trim().is_empty()) {
        return Err("empty browser document title".into());
    }
    let skip = driver.command(
        "POST",
        &format!("{prefix}/element"),
        json!({"using":"css selector","value":"a.mt-skip"}),
    )?;
    let skip = segment(&skip, ELEMENT)?;
    keyboard(driver, session, "\u{e004}")?;
    let active = driver.command("GET", &format!("{prefix}/element/active"), Value::Null)?;
    if segment(&active, ELEMENT)? != skip {
        return Err("first keyboard Tab did not focus the skip link".into());
    }
    for property in ["display", "visibility", "opacity"] {
        let value = driver.command(
            "GET",
            &format!("{prefix}/element/{skip}/css/{property}"),
            Value::Null,
        )?;
        observed["skip_css"][property] = value.clone();
        if !usable_css(property, &value) {
            return Err(format!("focused skip link has unusable {property}"));
        }
    }
    let rect = driver.command("GET", &format!("{prefix}/element/{skip}/rect"), Value::Null)?;
    observed["skip_rect"] = rect.clone();
    if ["width", "height"].iter().any(|key| {
        rect[key]
            .as_f64()
            .is_none_or(|n| !n.is_finite() || n <= 0.0)
    }) {
        return Err("focused skip link has no positive geometry".into());
    }
    let href = driver.command(
        "GET",
        &format!("{prefix}/element/{skip}/attribute/href"),
        Value::Null,
    )?;
    let target = href
        .as_str()
        .and_then(|href| href.strip_prefix('#'))
        .filter(|target| !target.is_empty())
        .ok_or("skip link has no local fragment")?;
    keyboard(driver, session, "\u{e007}")?;
    let active = driver.command("GET", &format!("{prefix}/element/active"), Value::Null)?;
    let active = segment(&active, ELEMENT)?;
    let id = driver.command(
        "GET",
        &format!("{prefix}/element/{active}/attribute/id"),
        Value::Null,
    )?;
    observed["active_target_id"] = id.clone();
    if id.as_str() != Some(target) {
        return Err("keyboard activation did not focus the skip target".into());
    }
    Ok(())
}
