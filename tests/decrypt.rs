//! End-to-end decryption tests against images encrypted by OpenSSL
//! (see tests/fixtures/make_fixtures.py).

use std::fs;
use std::path::PathBuf;

use ps3recover::decrypt::{self, Model};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn decrypt_fixture(image: &str, model: Option<Model>) -> anyhow::Result<(Model, Vec<u8>)> {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("decrypted.img");
    let used = decrypt::run(
        &fixture(image),
        &out,
        &fixture("fake_eid_root_key.bin"),
        model,
    )?;
    Ok((used, fs::read(out).unwrap()))
}

#[test]
fn decrypts_slim_image_and_detects_model() {
    let (model, data) = decrypt_fixture("slim.img", None).unwrap();
    assert_eq!(model, Model::Slim);
    assert_eq!(data, fs::read(fixture("plain.bin")).unwrap());
}

#[test]
fn decrypts_fat_image_and_detects_model() {
    let (model, data) = decrypt_fixture("fat.img", None).unwrap();
    assert_eq!(model, Model::Fat);
    assert_eq!(data, fs::read(fixture("plain.bin")).unwrap());
}

#[test]
fn wrong_model_is_refused() {
    let err = decrypt_fixture("slim.img", Some(Model::Fat)).unwrap_err();
    assert!(err.to_string().contains("could not be decrypted"), "{err}");
}

#[test]
fn wrong_key_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("other_key");
    fs::write(&key, [0x42u8; 48]).unwrap();
    let out = dir.path().join("decrypted.img");
    let err = decrypt::run(&fixture("slim.img"), &out, &key, None).unwrap_err();
    assert!(err.to_string().contains("could not be decrypted"), "{err}");
    assert!(
        !out.exists(),
        "no output should be written with a wrong key"
    );
}

#[test]
fn existing_output_is_never_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("decrypted.img");
    fs::write(&out, b"keep me").unwrap();
    let err = decrypt::run(
        &fixture("slim.img"),
        &out,
        &fixture("fake_eid_root_key.bin"),
        None,
    )
    .unwrap_err();
    assert!(err.to_string().contains("already exists"), "{err}");
    assert_eq!(fs::read(&out).unwrap(), b"keep me");
}
