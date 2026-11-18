use super::*;
use std::{
    io::BufReader,
    net::TcpListener,
    sync::{Arc, Mutex as StdMutex},
    thread,
};

fn parse(wire: &[u8]) -> Result<Reply, RedisError> {
    read_reply(&mut BufReader::new(wire), 0)
}

fn bulk(text: &str) -> Reply {
    Reply::Bulk(text.as_bytes().to_vec())
}

#[test]
fn command_encoding_is_a_resp_array_of_bulk_strings() {
    assert_eq!(
        encode(&[b"SET", b"k", b"v"]),
        b"*3\r\n$3\r\nSET\r\n$1\r\nk\r\n$1\r\nv\r\n"
    );
}

#[test]
fn scalar_replies_parse_and_bulk_is_length_delimited() {
    assert_eq!(parse(b"+OK\r\n").unwrap(), Reply::Simple("OK".into()));
    assert_eq!(parse(b":42\r\n").unwrap(), Reply::Integer(42));
    assert_eq!(parse(b"$5\r\nhello\r\n").unwrap(), bulk("hello"));
    assert_eq!(parse(b"$-1\r\n").unwrap(), Reply::Nil);
    assert_eq!(parse(b"*-1\r\n").unwrap(), Reply::Nil);
    assert_eq!(
        parse(b"$7\r\na\r\nb\r\nc\r\n").unwrap(),
        bulk("a\r\nb\r\nc")
    );
    assert_eq!(bulk("hello").text(), Some("hello"));
}

#[test]
fn error_reply_surfaces_as_a_server_error() {
    assert!(matches!(
        parse(b"-WRONGTYPE nope\r\n"),
        Err(RedisError::Server(message)) if message == "WRONGTYPE nope"
    ));
}

#[test]
fn scan_reply_parses_as_nested_array() {
    let reply = parse(b"*2\r\n$1\r\n0\r\n*2\r\n$9\r\nhmn:ban:a\r\n$9\r\nhmn:ban:b\r\n").unwrap();
    assert_eq!(
        reply,
        Reply::Array(vec![
            bulk("0"),
            Reply::Array(vec![bulk("hmn:ban:a"), bulk("hmn:ban:b")])
        ])
    );
}

#[test]
fn oversized_lengths_are_refused_before_allocation() {
    assert!(matches!(
        parse(b"$999999999\r\n"),
        Err(RedisError::Protocol(_))
    ));
    assert!(matches!(
        parse(b"*999999999\r\n"),
        Err(RedisError::Protocol(_))
    ));
    let long = [b"+".as_slice(), &vec![b'a'; MAX_LINE_LEN], b"\r\n"].concat();
    assert!(matches!(parse(&long), Err(RedisError::Protocol(_))));
}

#[test]
fn deep_nesting_is_refused() {
    let mut wire = b"*1\r\n".repeat(100);
    wire.extend_from_slice(b"$1\r\nx\r\n");
    assert!(matches!(parse(&wire), Err(RedisError::Protocol(_))));
    let mut wire = b"*1\r\n".repeat(MAX_DEPTH);
    wire.extend_from_slice(b"$1\r\nx\r\n");
    assert!(parse(&wire).is_ok());
}

#[test]
fn malformed_frames_are_refused() {
    for wire in [
        b"?x\r\n".as_slice(),
        b"+OK\n",
        b":nan\r\n",
        b"$3\r\nabcXY",
        b"",
    ] {
        assert!(parse(wire).is_err(), "{wire:?}");
    }
}

#[test]
fn breaker_trips_on_failure_and_fast_fails_within_cooldown() {
    // Bind then drop to obtain a closed local port.
    let address = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let client = Client::new(
        address.to_string(),
        ClientSettings {
            cooldown: Duration::from_secs(60),
            ..ClientSettings::default()
        },
    );
    assert!(matches!(client.command(&[b"PING"]), Err(RedisError::Io(_))));
    assert!(matches!(
        client.command(&[b"PING"]),
        Err(RedisError::BreakerOpen)
    ));
}

/// Commands a test server received, each as its argument list.
type Seen = Arc<StdMutex<Vec<Vec<Vec<u8>>>>>;

/// Minimal RESP server: records each command and answers from `answer`.
fn serve(answer: impl Fn(&[Vec<u8>]) -> Vec<u8> + Send + 'static) -> (String, Seen) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let seen = Arc::new(StdMutex::new(Vec::new()));
    let log = seen.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { return };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut writer = stream;
            while let Ok(Reply::Array(items)) = read_reply(&mut reader, 0) {
                let args: Vec<Vec<u8>> = items
                    .into_iter()
                    .map(|item| match item {
                        Reply::Bulk(bytes) => bytes,
                        _ => Vec::new(),
                    })
                    .collect();
                log.lock().unwrap().push(args.clone());
                if writer.write_all(&answer(&args)).is_err() {
                    break;
                }
            }
        }
    });
    (address, seen)
}

#[test]
fn auth_is_sent_on_connect_then_commands_reuse_the_connection() {
    let (address, seen) = serve(|args| match args[0].as_slice() {
        b"AUTH" => b"+OK\r\n".to_vec(),
        b"GET" => b"$1\r\nv\r\n".to_vec(),
        _ => b"-ERR unknown\r\n".to_vec(),
    });
    let client = Client::new(
        address,
        ClientSettings {
            user: Some("gate".into()),
            password: Some("secret".into()),
            ..ClientSettings::default()
        },
    );
    assert_eq!(client.command(&[b"GET", b"k"]).unwrap(), bulk("v"));
    assert_eq!(client.command(&[b"GET", b"k"]).unwrap(), bulk("v"));
    let seen = seen.lock().unwrap().clone();
    assert_eq!(
        seen,
        vec![
            vec![b"AUTH".to_vec(), b"gate".to_vec(), b"secret".to_vec()],
            vec![b"GET".to_vec(), b"k".to_vec()],
            vec![b"GET".to_vec(), b"k".to_vec()],
        ]
    );
    assert!(!format!("{client:?}").contains("secret"));
}

#[test]
fn rejected_auth_fails_the_command_and_opens_the_breaker() {
    let (address, _) = serve(|_| b"-WRONGPASS invalid\r\n".to_vec());
    let client = Client::new(
        address,
        ClientSettings {
            password: Some("bad".into()),
            ..ClientSettings::default()
        },
    );
    assert!(matches!(
        client.command(&[b"PING"]),
        Err(RedisError::Server(message)) if message.starts_with("WRONGPASS")
    ));
    assert!(matches!(
        client.command(&[b"PING"]),
        Err(RedisError::BreakerOpen)
    ));
}

#[test]
fn slow_success_returns_the_reply_but_opens_the_breaker() {
    let (address, _) = serve(|_| {
        thread::sleep(Duration::from_millis(60));
        b"+PONG\r\n".to_vec()
    });
    let client = Client::new(
        address,
        ClientSettings {
            slow: Duration::from_millis(20),
            slow_ewma: Duration::ZERO,
            cooldown: Duration::from_secs(60),
            ..ClientSettings::default()
        },
    );
    assert_eq!(
        client.command(&[b"PING"]).unwrap(),
        Reply::Simple("PONG".into())
    );
    assert!(matches!(
        client.command(&[b"PING"]),
        Err(RedisError::BreakerOpen)
    ));
}

#[test]
fn sustained_latency_trips_through_the_ewma() {
    let (address, _) = serve(|_| {
        thread::sleep(Duration::from_millis(30));
        b"+PONG\r\n".to_vec()
    });
    let client = Client::new(
        address,
        ClientSettings {
            slow: Duration::ZERO,
            slow_ewma: Duration::from_millis(20),
            cooldown: Duration::from_secs(60),
            ..ClientSettings::default()
        },
    );
    // Each 30 ms reply moves the EWMA 40% toward 30 ms; the second pushes it
    // past 20 ms (12 ms, then 19.2 ms, then 23.5 ms).
    for _ in 0..3 {
        if client.command(&[b"PING"]).is_err() {
            break;
        }
    }
    assert!(matches!(
        client.command(&[b"PING"]),
        Err(RedisError::BreakerOpen)
    ));
}

#[test]
fn a_dropped_connection_is_redialled_after_the_cooldown() {
    let (address, seen) = serve(|args| {
        if args[0] == b"QUIT" {
            Vec::new()
        } else {
            b"+PONG\r\n".to_vec()
        }
    });
    let client = Client::new(
        address,
        ClientSettings {
            cooldown: Duration::ZERO,
            ..ClientSettings::default()
        },
    );
    assert!(client.command(&[b"PING"]).is_ok());
    client.close();
    assert!(client.command(&[b"PING"]).is_ok());
    assert_eq!(seen.lock().unwrap().len(), 2);
}
