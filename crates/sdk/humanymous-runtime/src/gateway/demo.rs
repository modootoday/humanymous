//! Public "score this browser" demo page (legacy cmd/server handleDemo at
//! baseline 981d25c).
//!
//! The page is read-only and self-scoring: no launcher and no target field.
//! It is served without the development playground flag, and it issues the
//! `hsid` session cookie so the WASM client, `/api/collect` and
//! `/api/report/{sid}` flow has a session to attach to.
use super::observatory::{Body, Request, Response};
use std::{io, path::Path};

/// Route the demo page is served on.
pub const PATH: &str = "/demo";
/// Legacy demo session cookie name.
pub const SESSION_COOKIE: &str = "hsid";

/// Session id from the `hsid` cookie when present and non-empty.
pub fn session_from_cookie(header: Option<&str>) -> Option<&str> {
    header?
        .split(';')
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == SESSION_COOKIE)
        .map(|(_, value)| value.trim_matches('"'))
        .filter(|value| !value.is_empty())
}

/// Fresh 128-bit session id. A failed CSPRNG fails the request instead of
/// yielding a guessable id (SoT-31 R4).
pub fn new_session() -> io::Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| io::Error::other("session entropy unavailable"))?;
    Ok(hex::encode(bytes))
}

/// `Set-Cookie` value Go's `http.SetCookie` wrote for a new session.
pub fn session_cookie(session: &str) -> String {
    format!("{SESSION_COOKIE}={session}; Path=/; HttpOnly; Secure; SameSite=Lax")
}

/// Serve `web_dir/demo.html` for [`PATH`], reusing or issuing the session and
/// registering it through `ensure`. `None` for any other path.
pub fn handle(
    request: &Request<'_>,
    web_dir: &Path,
    ensure: &dyn Fn(&str),
) -> Option<io::Result<Response>> {
    if request.path != PATH {
        return None;
    }
    let mut headers = Vec::new();
    let session = match session_from_cookie(request.cookie) {
        Some(session) => session.to_owned(),
        None => match new_session() {
            Ok(session) => {
                headers.push(("Set-Cookie", session_cookie(&session)));
                session
            }
            Err(error) => return Some(Err(error)),
        },
    };
    ensure(&session);
    Some(Ok(Response {
        status: 200,
        headers,
        body: Body::File(web_dir.join("demo.html")),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::observatory::Observatory;
    use humanymous_core::scoring::engine::Engine;
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    fn web_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../web")
    }

    fn demo(cookie: Option<&str>) -> (Response, Vec<String>) {
        let seen = Mutex::new(Vec::new());
        let request = Request {
            method: "GET",
            path: PATH,
            host: "example.test",
            cookie,
            ..Request::default()
        };
        let response = handle(&request, &web_dir(), &|session| {
            seen.lock().unwrap().push(session.to_owned());
        })
        .unwrap()
        .unwrap();
        (response, seen.into_inner().unwrap())
    }

    #[test]
    fn demo_page_is_served() {
        let (response, _) = demo(None);
        assert_eq!(response.status, 200);
        let Body::File(path) = &response.body else {
            panic!("expected the demo file");
        };
        assert_eq!(path, &web_dir().join("demo.html"));
        let page = std::fs::read_to_string(path).unwrap();
        assert!(page.contains("Score this browser"), "missing the score CTA");
        assert!(
            page.contains("/static/js/demo.js"),
            "must load the demo module"
        );
    }

    #[test]
    fn demo_issues_and_reuses_a_session() {
        let (response, ensured) = demo(None);
        let cookie = response.header("Set-Cookie").unwrap();
        let session = cookie
            .strip_prefix("hsid=")
            .and_then(|rest| rest.split(';').next())
            .unwrap();
        assert_eq!(session.len(), 32);
        assert!(session.bytes().all(|b| b.is_ascii_hexdigit()));
        assert!(cookie.ends_with("; Path=/; HttpOnly; Secure; SameSite=Lax"));
        assert_eq!(ensured, [session]);
        let (again, ensured) = demo(Some("theme=dark; hsid=abc123"));
        assert_eq!(again.header("Set-Cookie"), None);
        assert_eq!(ensured, ["abc123"]);
        let (fresh, _) = demo(Some("hsid="));
        assert!(fresh.header("Set-Cookie").is_some());
        assert_ne!(new_session().unwrap(), new_session().unwrap());
    }

    #[test]
    fn demo_is_independent_of_the_playground() {
        let playground = Observatory::new(
            Some("0"),
            "127.0.0.1:8443",
            web_dir(),
            Arc::new(Engine::default()),
            Arc::new(NoSessions),
            Arc::new(|_, _, _| Ok(String::new())),
            Arc::new(|| {}),
        )
        .unwrap();
        assert!(
            playground.is_none(),
            "playground routes are absent when off"
        );
        assert!(demo(None).0.status == 200);
        let other = Request {
            path: "/playground",
            ..Request::default()
        };
        assert!(handle(&other, &web_dir(), &|_| {}).is_none());
    }

    struct NoSessions;
    impl crate::gateway::observatory::Sessions for NoSessions {
        fn report(&self, _: &str) -> Option<humanymous_core::signals::report::SessionReport> {
            None
        }
    }
}
