use rscleaner_model::{ErrorCode, Platform, RuleError, RuleFile};
use rscleaner_platform::{FakeEnv, FakeKnownFolders};
use rscleaner_rules::validate::{ValidationOptions, validate_file_with_text};

fn validate(yaml: &str, opts: &ValidationOptions) -> Vec<RuleError> {
    let file: RuleFile = match serde_norway::from_str(yaml) {
        Ok(file) => file,
        Err(error) => panic!("fixture parses: {error}"),
    };
    validate_file_with_text(
        &file,
        yaml,
        &FakeEnv::default(),
        &FakeKnownFolders::default(),
        opts,
    )
}

fn invalid_cases() -> Vec<(&'static str, String, ErrorCode)> {
    let base = "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n";
    vec![
        ("unknown-var", format!("{base}      - path: ${{NOPE}}/x\n"), ErrorCode::UnknownVar),
        ("empty-targets", "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets: []\n".into(), ErrorCode::EmptyTargets),
        ("bad-glob", format!("{base}      - path: C:/Temp\n        include: [\"[\"]\n"), ErrorCode::BadGlob),
        ("registry-user", format!("{base}      - kind: registry\n        check: reg.test\n        hive: HKCU\n        key: Software/Test\n        value: null\n"), ErrorCode::RegistryUser),
        ("risk-user", "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: user\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::RiskUnderstated),
        ("command-disabled", format!("{base}      - kind: command\n        cmd: cmd.exe\n"), ErrorCode::CommandDisabled),
        ("platform-when", "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    when:\n      distro: [ubuntu]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::PlatformWhen),
        ("empty-include", format!("{base}      - path: C:/Temp\n        include: [\"\"]\n"), ErrorCode::EmptyMask),
        ("empty-exclude", format!("{base}      - path: C:/Temp\n        exclude: [\"\"]\n"), ErrorCode::EmptyMask),
        ("invalid-id", "schema: 1\nrules:\n  - id: Bad.Rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::IdPattern),
        ("path-empty", format!("{base}      - path: \"\"\n"), ErrorCode::Schema),
        ("find-empty-name", format!("{base}      - kind: find\n        name: \"\"\n"), ErrorCode::Schema),
        ("find-max-depth-zero", format!("{base}      - kind: find\n        name: node_modules\n        max_depth: 0\n"), ErrorCode::Schema),
        ("command-empty", format!("{base}      - kind: command\n        cmd: \"\"\n"), ErrorCode::Schema),
        ("command-timeout-zero", format!("{base}      - kind: command\n        cmd: cmd.exe\n        timeout_s: 0\n"), ErrorCode::Schema),
        ("risk-command", format!("{base}      - kind: command\n        cmd: cmd.exe\n"), ErrorCode::RiskUnderstated),
        ("risk-find", format!("{base}      - kind: find\n        name: node_modules\n"), ErrorCode::RiskUnderstated),
        ("risk-user-path", format!("{base}      - path: ${{USERPROFILE}}/Documents/file.txt\n"), ErrorCode::RiskUnderstated),
        ("find-unknown-root", format!("{base}      - kind: find\n        roots: [\"${{NOPE}}\"]\n        name: node_modules\n"), ErrorCode::UnknownVar),
        ("duplicate-id", "schema: 1\nrules:\n  - id: test.rule\n    name: One\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n  - id: test.rule\n    name: Two\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::DuplicateId),
        ("id-one-segment", "schema: 1\nrules:\n  - id: browsers\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::IdPattern),
        ("id-uppercase", "schema: 1\nrules:\n  - id: Browsers.chrome\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::IdPattern),
        ("id-hyphen-first", "schema: 1\nrules:\n  - id: bad-id.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::IdPattern),
        ("id-empty-segment", "schema: 1\nrules:\n  - id: test..rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::IdPattern),
        ("bad-glob-brace", format!("{base}      - path: C:/Temp\n        include: [\"{{a,\"]\n"), ErrorCode::BadGlob),
        ("bad-glob-range", format!("{base}      - path: C:/Temp\n        exclude: [\"[z-a]\"]\n"), ErrorCode::BadGlob),
        ("empty-include-two", format!("{base}      - path: C:/Temp\n        include: [\"*.tmp\", \"\"]\n"), ErrorCode::EmptyMask),
        ("empty-exclude-two", format!("{base}      - path: C:/Temp\n        exclude: [\"\", \"keep\"]\n"), ErrorCode::EmptyMask),
        ("risk-user-find", "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: user\n    risk: moderate\n    platforms: [windows]\n    targets:\n      - kind: find\n        name: node_modules\n".into(), ErrorCode::RiskUnderstated),
        ("command-disabled-args", format!("{base}      - kind: command\n        cmd: cmd.exe\n        args: [\"/c\", \"echo\"]\n"), ErrorCode::CommandDisabled),
        ("registry-user-value", format!("{base}      - kind: registry\n        check: reg.test\n        hive: HKCU\n        key: Software/Test\n        value: Probe\n"), ErrorCode::RegistryUser),
        ("find-depth-zero-roots", format!("{base}      - kind: find\n        roots: [C:/Projects]\n        name: node_modules\n        max_depth: 0\n"), ErrorCode::Schema),
        ("path-empty-include", format!("{base}      - path: \"\"\n        include: [\"*.tmp\"]\n"), ErrorCode::Schema),
        ("find-empty-name-sibling", format!("{base}      - kind: find\n        name: \"\"\n        has_sibling: [package.json]\n"), ErrorCode::Schema),
        ("command-empty-args", format!("{base}      - kind: command\n        cmd: \"\"\n        args: [\"/c\"]\n"), ErrorCode::Schema),
        ("command-timeout-zero-args", format!("{base}      - kind: command\n        cmd: cmd.exe\n        args: [\"/c\"]\n        timeout_s: 0\n"), ErrorCode::Schema),
        ("platform-when-distro", "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    when:\n      distro: [fedora]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::PlatformWhen),
        ("duplicate-id-three", "schema: 1\nrules:\n  - id: test.rule\n    name: One\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n  - id: test.rule\n    name: Two\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n  - id: test.rule\n    name: Three\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n".into(), ErrorCode::DuplicateId),
        ("find-unknown-root-two", format!("{base}      - kind: find\n        roots: [\"${{NOPE}}\", C:/Projects]\n        name: node_modules\n"), ErrorCode::UnknownVar),
        ("unknown-var-two", format!("{base}      - path: \"${{NOPE1}}/${{NOPE2}}\"\n"), ErrorCode::UnknownVar),
        ("empty-targets-two", "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets: []\n".into(), ErrorCode::EmptyTargets),
        ("empty-include-bad-exclude", format!("{base}      - path: C:/Temp\n        include: [\"\"]\n        exclude: [\"[\"]\n"), ErrorCode::EmptyMask),
    ]
}

#[test]
fn invalid_cases_report_their_codes_and_positions() {
    for (name, yaml, code) in invalid_cases() {
        let errors = validate(&yaml, &ValidationOptions::default());
        let found = errors.iter().find(|error| error.code == code);
        assert!(found.is_some(), "{name}: expected {code:?}, got {errors:?}");
        assert_eq!(found.map(|error| error.line), Some(3), "{name}");
    }
}

#[test]
fn linux_os_build_is_rejected() {
    let yaml = "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [linux]\n    os_min_build: 19045\n    targets:\n      - path: /tmp/cache\n";
    let opts = ValidationOptions {
        platform: Platform::Linux,
        ..ValidationOptions::default()
    };
    let errors = validate(yaml, &opts);
    assert!(
        errors
            .iter()
            .any(|error| error.code == ErrorCode::PlatformWhen)
    );
}

#[test]
fn valid_rules_produce_no_errors() {
    let cases = [
        "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n        include: [\"*.tmp\"]\n        exclude: [\"keep.tmp\"]\n".to_string(),
        "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: ${LOCALAPPDATA}/Temp\n".to_string(),
        "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: moderate\n    platforms: [windows]\n    targets:\n      - kind: command\n        cmd: cmd.exe\n        args: [\"/c\", \"echo ok\"]\n".to_string(),
        "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: test\n    risk: moderate\n    platforms: [windows]\n    targets:\n      - kind: find\n        name: node_modules\n        has_sibling: [package.json]\n".to_string(),
        "schema: 1\nrules:\n  - id: test.rule\n    name: {en: Test, ru: Тест}\n    category: test\n    risk: safe\n    platforms: [windows]\n    targets:\n      - path: C:/Temp\n        min_size: 50MB\n".to_string(),
        "schema: 1\nrules: []\n".into(),
    ];
    let opts = ValidationOptions {
        allow_user_commands: true,
        ..ValidationOptions::default()
    };
    for yaml in cases {
        assert!(validate(&yaml, &opts).is_empty(), "{yaml}");
    }
}

#[test]
fn allowed_registry_target_is_valid_on_windows() {
    let yaml = "schema: 1\nrules:\n  - id: test.rule\n    name: Test\n    category: traces\n    risk: safe\n    platforms: [windows]\n    targets:\n      - kind: registry\n        check: reg.test\n        hive: HKCU\n        key: Software/Test\n        value: null\n";
    let opts = ValidationOptions {
        allow_registry: true,
        ..ValidationOptions::default()
    };
    assert!(validate(yaml, &opts).is_empty());
}

#[test]
fn risk_inference_does_not_depend_on_the_environment() {
    let gradle = "schema: 1\nrules:\n  - id: dev.gradle-caches\n    name: Test\n    category: dev\n    risk: moderate\n    platforms: [windows]\n    targets:\n      - path: '${USERPROFILE}\\.gradle\\caches'\n";
    let downloads = "schema: 1\nrules:\n  - id: downloads.old-installers\n    name: Test\n    category: downloads\n    risk: moderate\n    platforms: [windows]\n    targets:\n      - path: '${USERPROFILE}\\Downloads'\n        include: ['*.exe']\n        min_age_days: 30\n";
    let windows_env = FakeEnv::from([("USERPROFILE", r"C:\Users\Me")]);

    for env in [&FakeEnv::default(), &windows_env] {
        let known = FakeKnownFolders::default();
        let opts = ValidationOptions::default();
        let gradle_file: RuleFile = serde_norway::from_str(gradle).expect("gradle fixture parses");
        let downloads_file: RuleFile =
            serde_norway::from_str(downloads).expect("downloads fixture parses");

        let gradle_errors = validate_file_with_text(&gradle_file, gradle, env, &known, &opts);
        assert!(
            gradle_errors
                .iter()
                .all(|error| error.code != ErrorCode::RiskUnderstated),
            "gradle cache is not user data: {gradle_errors:?}"
        );

        let downloads_errors =
            validate_file_with_text(&downloads_file, downloads, env, &known, &opts);
        assert!(
            downloads_errors
                .iter()
                .any(|error| error.code == ErrorCode::RiskUnderstated),
            "downloads is user data on every platform: {downloads_errors:?}"
        );
    }
}
