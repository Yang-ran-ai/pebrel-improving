        let fixtures = [
            (
                "Responses",
                "/responses",
                "authorization: bearer",
                ("reasoning", "effort", "none"),
                r#"{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"responses ok"}]}]}"#,
            ),
            (
                "Chat Completions",
                "/chat/completions",
                "authorization: bearer",
                ("thinking", "type", "disabled"),
                r#"{"choices":[{"message":{"content":"chat ok"}}]}"#,
            ),
            (
                "Claude Messages",
                "/anthropic/v1/messages",
                "x-api-key: test-deepseek-key",
                ("reasoning", "effort", "none"),
                r#"{"content":[{"type":"text","text":"messages ok"}]}"#,
            ),
        ];
        for (protocol, route, expected_auth, (option_group, option_name, option_value), response_body) in fixtures {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let mut provider = AiProvider::preset(ProviderKind::DeepSeek, "deepseek-http-probe");
            provider.base_url = format!("http://{}", listener.local_addr().unwrap());
            provider.wire_api = protocol.into();
            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0u8; 1024];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    assert_ne!(count, 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]);
                        let length: usize = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse().unwrap())
                            })
                            .unwrap();
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8_lossy(&bytes);
                assert!(
                    request.to_ascii_lowercase().starts_with(&format!("post {route} http/1.1")),
                    "{protocol} request route: {request}"
                );
                assert!(
                    request.to_ascii_lowercase().contains(expected_auth),
                    "{protocol} auth header missing"
                );
                let header_end = bytes
                    .windows(4)
                    .position(|window| window == b"\r\n\r\n")
                    .unwrap();
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length: usize = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse().unwrap())
                    })
                    .unwrap();
                let body_start = header_end + 4;
                let body: serde_json::Value =
                    serde_json::from_slice(&bytes[body_start..body_start + content_length])
                        .unwrap();
                assert_eq!(
                    body[option_group][option_name].as_str(),
                    Some(option_value),
                    "{protocol} request body option is incorrect: {body}"
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
                    response_body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            });
            let result = test_deepseek_generation(
                &provider,
                Some(Zeroizing::new("test-deepseek-key".to_owned())),
            )
            .unwrap();
            assert_eq!(result.0, 200, "{protocol}");
            assert!(
                result.1.ends_with("ok"),
                "{protocol}: {}",
                result.1
            );
            server.join().unwrap();
        }
    }

}
