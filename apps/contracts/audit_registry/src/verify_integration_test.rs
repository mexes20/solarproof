//! Integration tests for `audit_registry::verify` (#563).
//!
//! Tests the full anchor → verify round-trip including ledger sequence
//! recording, field correctness, None for unanchored hashes, and independence
//! across multiple hashes.

#![cfg(test)]

use crate::{AnchorStatus, AuditRegistry, AuditRegistryClient, Error};
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, BytesN, Env};

fn setup() -> (Env, Address, AuditRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(AuditRegistry, ());
    let client = AuditRegistryClient::new(&env, &id);
    let admin = Address::generate(&env);
    let api_signer = Address::generate(&env);
    client.initialize(&admin, &api_signer);
    (env, api_signer, client)
}

fn make_hash(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

fn make_nonce(env: &Env, val: u8) -> BytesN<32> {
    BytesN::from_array(env, &[val; 32])
}

// ── verify returns None before anchoring ─────────────────────────────────────

#[test]
fn verify_returns_none_for_unanchored_hash() {
    let (env, _signer, client) = setup();
    assert!(client.verify(&make_hash(&env, 0x01)).is_none());
}

// ── verify returns correct fields after anchoring ────────────────────────────

#[test]
fn verify_returns_anchor_with_correct_reading_hash() {
    let (env, signer, client) = setup();
    let h = make_hash(&env, 0x42);
    client.anchor(&signer, &h, &make_nonce(&env, 1));
    let anchor = client.verify(&h).expect("must be Some after anchor");
    assert_eq!(anchor.reading_hash, h);
}

#[test]
fn verify_records_ledger_sequence_at_anchor_time() {
    let (env, signer, client) = setup();
    env.ledger().set_sequence_number(999);
    let h = make_hash(&env, 0x10);
    client.anchor(&signer, &h, &make_nonce(&env, 1));
    let anchor = client.verify(&h).expect("must be Some");
    assert_eq!(anchor.anchored_at_ledger, 999);
}

#[test]
fn verify_ledger_sequence_is_not_current_ledger_if_anchored_earlier() {
    let (env, signer, client) = setup();
    env.ledger().set_sequence_number(100);
    let h = make_hash(&env, 0x20);
    client.anchor(&signer, &h, &make_nonce(&env, 1));
    env.ledger().set_sequence_number(500);
    let anchor = client.verify(&h).expect("must be Some");
    assert_eq!(anchor.anchored_at_ledger, 100);
}

// ── verify is idempotent ──────────────────────────────────────────────────────

#[test]
fn verify_is_idempotent() {
    let (env, signer, client) = setup();
    let h = make_hash(&env, 0x55);
    client.anchor(&signer, &h, &make_nonce(&env, 1));
    let a1 = client.verify(&h).expect("first verify must be Some");
    let a2 = client.verify(&h).expect("second verify must be Some");
    assert_eq!(a1.reading_hash, a2.reading_hash);
    assert_eq!(a1.anchored_at_ledger, a2.anchored_at_ledger);
}

// ── verify distinguishes independent hashes ──────────────────────────────────

#[test]
fn verify_distinguishes_two_hashes() {
    let (env, signer, client) = setup();
    let h1 = make_hash(&env, 0xAA);
    let h2 = make_hash(&env, 0xBB);
    env.ledger().set_sequence_number(1);
    client.anchor(&signer, &h1, &make_nonce(&env, 1));
    env.ledger().set_sequence_number(2);
    client.anchor(&signer, &h2, &make_nonce(&env, 2));

    let a1 = client.verify(&h1).expect("h1 must be anchored");
    let a2 = client.verify(&h2).expect("h2 must be anchored");
    assert_eq!(a1.reading_hash, h1);
    assert_eq!(a2.reading_hash, h2);
    assert_eq!(a1.anchored_at_ledger, 1);
    assert_eq!(a2.anchored_at_ledger, 2);
}

// ── verify after duplicate anchor attempt ────────────────────────────────────

#[test]
fn verify_unchanged_after_rejected_duplicate_anchor() {
    let (env, signer, client) = setup();
    env.ledger().set_sequence_number(10);
    let h = make_hash(&env, 0x77);
    client.anchor(&signer, &h, &make_nonce(&env, 1));

    env.ledger().set_sequence_number(20);
    assert_eq!(
        client.try_anchor(&signer, &h, &make_nonce(&env, 2)),
        Err(Ok(Error::AlreadyAnchored))
    );

    let anchor = client.verify(&h).expect("must still be anchored");
    assert_eq!(anchor.anchored_at_ledger, 10);
}

// ── anchor_status helper view ────────────────────────────────────────────────

#[test]
fn anchor_status_matches_verify() {
    let (env, signer, client) = setup();
    let h = make_hash(&env, 0x33);
    assert_eq!(
        client.anchor_status(&h),
        AnchorStatus {
            is_anchored: false,
            anchored_at_ledger: 0,
        }
    );

    env.ledger().set_sequence_number(77);
    client.anchor(&signer, &h, &make_nonce(&env, 1));
    assert_eq!(
        client.anchor_status(&h),
        AnchorStatus {
            is_anchored: true,
            anchored_at_ledger: 77,
        }
    );
    assert_eq!(client.anchored_at_ledger(&h), Some(77));
}

// ── boundary hash values ──────────────────────────────────────────────────────

#[test]
fn verify_works_with_all_zeros_hash() {
    let (env, signer, client) = setup();
    let h = BytesN::from_array(&env, &[0x00u8; 32]);
    client.anchor(&signer, &h, &make_nonce(&env, 1));
    assert!(client.verify(&h).is_some());
}

#[test]
fn verify_works_with_all_ones_hash() {
    let (env, signer, client) = setup();
    let h = BytesN::from_array(&env, &[0xFFu8; 32]);
    client.anchor(&signer, &h, &make_nonce(&env, 1));
    assert!(client.verify(&h).is_some());
}
