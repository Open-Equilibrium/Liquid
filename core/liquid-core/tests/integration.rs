//! Integration tests for `liquid-core` primitives.
//!
//! These tests live outside the crate so they exercise only the public API.
//! Every type in `liquid-core` is touched here at least once.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bytes::Bytes;
use liquid_core::{
    Action, AppInstanceId, CommitId, ComponentId, ContentHash, LiquidError, OperationId, PageId,
    PrincipalId, Resource, RoleId, SlotName, SlotValue, StorePath, TenantConfig, WorkspaceId,
};

// ── ID newtypes ──────────────────────────────────────────────────────────────

#[test]
fn workspace_id_new_is_unique() {
    let a = WorkspaceId::new();
    let b = WorkspaceId::new();
    assert_ne!(a, b, "WorkspaceId::new must produce unique values");
}

#[test]
fn id_newtypes_round_trip_via_serde_json() {
    let cases: Vec<String> = vec![
        serde_json::to_string(&WorkspaceId::new()).expect("ws"),
        serde_json::to_string(&AppInstanceId::new()).expect("app"),
        serde_json::to_string(&ComponentId::new()).expect("cmp"),
        serde_json::to_string(&PageId::new()).expect("page"),
        serde_json::to_string(&RoleId::new()).expect("role"),
        serde_json::to_string(&OperationId::new()).expect("op"),
        serde_json::to_string(&CommitId::new()).expect("commit"),
    ];
    for s in &cases {
        // serde(transparent) → quoted UUID, length 38 (36 + 2 quotes).
        assert_eq!(s.len(), 38, "expected serde(transparent) UUID, got {s}");
    }
    let ws = WorkspaceId::new();
    let parsed: WorkspaceId =
        serde_json::from_str(&serde_json::to_string(&ws).expect("ser")).expect("round-trip");
    assert_eq!(ws, parsed);
}

#[test]
fn principal_id_distinguishes_user_and_agent() {
    let user = PrincipalId::new_user();
    let agent = PrincipalId::new_agent();
    assert!(!matches!(user, PrincipalId::Agent(_)));
    assert!(matches!(agent, PrincipalId::Agent(_)));
    assert!(agent.is_agent());
    assert!(!user.is_agent());
}

#[test]
fn principal_id_display_is_prefixed() {
    let user = PrincipalId::new_user();
    let agent = PrincipalId::new_agent();
    assert!(user.to_string().starts_with("user:"));
    assert!(agent.to_string().starts_with("agent:"));
}

#[test]
fn principal_id_round_trips_via_serde() {
    let p = PrincipalId::new_agent();
    let json = serde_json::to_string(&p).expect("ser");
    let back: PrincipalId = serde_json::from_str(&json).expect("de");
    assert_eq!(p, back);
}

// ── ContentHash ──────────────────────────────────────────────────────────────

#[test]
fn content_hash_accepts_64_lowercase_hex() {
    let valid = "0".repeat(64);
    let h = ContentHash::from_hex(&valid).expect("64 zeros are a valid hash");
    assert_eq!(h.as_str(), valid);
}

#[test]
fn content_hash_rejects_wrong_length() {
    assert!(matches!(
        ContentHash::from_hex("abc"),
        Err(LiquidError::InvalidInput(_))
    ));
    assert!(matches!(
        ContentHash::from_hex("a".repeat(63)),
        Err(LiquidError::InvalidInput(_))
    ));
    assert!(matches!(
        ContentHash::from_hex("a".repeat(65)),
        Err(LiquidError::InvalidInput(_))
    ));
}

#[test]
fn content_hash_rejects_non_hex_or_uppercase() {
    assert!(ContentHash::from_hex("g".repeat(64)).is_err());
    assert!(ContentHash::from_hex("A".repeat(64)).is_err());
    assert!(ContentHash::from_hex(format!("{}{}", "a".repeat(63), " ")).is_err());
}

#[test]
fn content_hash_of_bytes_is_known_sha256_for_empty_input() {
    // RFC 6234 vector: SHA-256("") =
    // e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
    let h = ContentHash::of_bytes(b"");
    assert_eq!(
        h.as_str(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn content_hash_of_bytes_is_known_sha256_for_abc() {
    // RFC 6234 vector: SHA-256("abc") =
    // ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
    let h = ContentHash::of_bytes(b"abc");
    assert_eq!(
        h.as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn content_hash_of_bytes_round_trips_through_from_hex() {
    // The output of `of_bytes` must always be a valid `from_hex`
    // input — keeps the two constructors interchangeable.
    let bytes = b"Liquid content-addressable cache test";
    let direct = ContentHash::of_bytes(bytes);
    let round = ContentHash::from_hex(direct.as_str()).expect("of_bytes output must validate");
    assert_eq!(direct, round);
}

#[test]
fn content_hash_of_bytes_is_collision_free_for_distinct_inputs() {
    assert_ne!(
        ContentHash::of_bytes(b"a"),
        ContentHash::of_bytes(b"b"),
        "distinct inputs must produce distinct hashes"
    );
}

// ── StorePath ────────────────────────────────────────────────────────────────

#[test]
fn store_path_accepts_simple_relative() {
    let p = StorePath::new("pages/index.json").expect("valid");
    assert_eq!(p.as_str(), "pages/index.json");
}

#[test]
fn store_path_rejects_empty() {
    assert!(matches!(
        StorePath::new(""),
        Err(LiquidError::InvalidInput(_))
    ));
}

#[test]
fn store_path_rejects_absolute() {
    assert!(StorePath::new("/etc/passwd").is_err());
}

#[test]
fn store_path_rejects_dotdot_anywhere() {
    assert!(StorePath::new("..").is_err());
    assert!(StorePath::new("../etc").is_err());
    assert!(StorePath::new("a/../b").is_err());
    assert!(StorePath::new("a/b/..").is_err());
}

#[test]
fn store_path_rejects_dot_segment() {
    assert!(StorePath::new(".").is_err());
    assert!(StorePath::new("a/./b").is_err());
}

#[test]
fn store_path_rejects_empty_segment() {
    assert!(StorePath::new("a//b").is_err());
}

#[test]
fn store_path_rejects_backslash_or_nul() {
    assert!(StorePath::new("a\\b").is_err());
    assert!(StorePath::new("a\0b").is_err());
}

// ── SlotName ─────────────────────────────────────────────────────────────────

#[test]
fn slot_name_accepts_namespace_descriptor() {
    let s = SlotName::new("sheet:selectedRange").expect("valid");
    assert_eq!(s.as_str(), "sheet:selectedRange");
}

#[test]
fn slot_name_rejects_missing_colon() {
    assert!(SlotName::new("noColonHere").is_err());
}

#[test]
fn slot_name_rejects_extra_colons() {
    assert!(SlotName::new("a:b:c").is_err());
}

#[test]
fn slot_name_rejects_empty_segments() {
    assert!(SlotName::new(":foo").is_err());
    assert!(SlotName::new("foo:").is_err());
    assert!(SlotName::new(":").is_err());
}

#[test]
fn slot_name_rejects_illegal_characters() {
    assert!(SlotName::new("sheet:selected range").is_err());
    assert!(SlotName::new("sheet:selected-range").is_err());
    assert!(SlotName::new("sheet:selected.range").is_err());
}

// ── SlotValue ────────────────────────────────────────────────────────────────

#[test]
fn slot_value_round_trips_each_variant() {
    let cases: Vec<SlotValue> = vec![
        SlotValue::Str("hello".into()),
        SlotValue::Num(2.5),
        SlotValue::Bool(true),
        SlotValue::Json(serde_json::json!({"k": [1, 2, 3]})),
        SlotValue::Bytes(Bytes::from_static(b"\x00\x01\xff")),
    ];
    for v in cases {
        let s = serde_json::to_string(&v).expect("ser");
        let back: SlotValue = serde_json::from_str(&s).expect("de");
        assert_eq!(v, back, "round-trip failed for {v:?}");
    }
}

// ── Action / Resource ────────────────────────────────────────────────────────

#[test]
fn action_serializes_lowercase() {
    assert_eq!(
        serde_json::to_string(&Action::Read).expect("ser"),
        "\"read\""
    );
    assert_eq!(
        serde_json::to_string(&Action::Write).expect("ser"),
        "\"write\""
    );
    assert_eq!(
        serde_json::to_string(&Action::Delete).expect("ser"),
        "\"delete\""
    );
    assert_eq!(
        serde_json::to_string(&Action::Admin).expect("ser"),
        "\"admin\""
    );
}

#[test]
fn resource_round_trips() {
    let resources = vec![
        Resource::Workspace(WorkspaceId::new()),
        Resource::AppInstance(AppInstanceId::new()),
        Resource::Component(ComponentId::new()),
        Resource::Page(PageId::new()),
        Resource::Field("revenue.may".into()),
    ];
    for r in resources {
        let s = serde_json::to_string(&r).expect("ser");
        let back: Resource = serde_json::from_str(&s).expect("de");
        assert_eq!(r, back);
    }
}

// ── TenantConfig ─────────────────────────────────────────────────────────────

#[test]
fn tenant_config_empty_is_object() {
    let cfg = TenantConfig::empty();
    assert!(cfg.as_value().is_object());
    assert_eq!(cfg.as_value().as_object().expect("obj").len(), 0);
}

#[test]
fn tenant_config_round_trips() {
    let cfg = TenantConfig(serde_json::json!({"apiUrl": "https://eu.example.com"}));
    let s = serde_json::to_string(&cfg).expect("ser");
    let back: TenantConfig = serde_json::from_str(&s).expect("de");
    assert_eq!(cfg, back);
}

// ── LiquidError ──────────────────────────────────────────────────────────────

#[test]
fn liquid_error_display_formats() {
    assert_eq!(LiquidError::Forbidden.to_string(), "forbidden");
    assert_eq!(
        LiquidError::InvalidInput("bad".into()).to_string(),
        "invalid input: bad"
    );
    assert_eq!(
        LiquidError::NotFound("page/x".into()).to_string(),
        "not found: page/x"
    );
}
