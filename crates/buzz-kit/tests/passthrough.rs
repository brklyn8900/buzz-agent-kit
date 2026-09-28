use buzz_kit::buzz::validate_read;
#[test]
fn read_allowlist_is_exact_and_cannot_override_identity_or_publish() {
    for pair in [
        "channels list",
        "channels get",
        "channels search",
        "channels members",
        "messages get",
        "messages thread",
        "messages search",
        "users get",
        "users presence",
        "canvas get",
        "canvas history",
        "repos get",
        "repos list",
        "feed get",
        "dms list",
    ] {
        let args: Vec<String> = pair.split(' ').map(String::from).collect();
        assert!(validate_read(&args).is_ok(), "{pair}");
        for forbidden in [
            "--broadcast",
            "--broadcast=true",
            "--private-key",
            "--private-key=hidden",
            "--relay=https://example.com",
            "--auth-tag=hidden",
            "--format=compact",
            "--help",
            "-h",
        ] {
            let mut bad = args.clone();
            bad.push(forbidden.into());
            assert!(validate_read(&bad).is_err(), "{forbidden}");
        }
    }
    for pair in [
        "messages send",
        "messages edit",
        "messages delete",
        "upload file",
        "canvas set",
        "social publish",
        "users set-profile",
        "channels add-member",
        "dms open",
        "channels",
        "unknown read",
    ] {
        assert!(
            validate_read(&pair.split(' ').map(String::from).collect::<Vec<_>>()).is_err(),
            "{pair}"
        );
    }
}
