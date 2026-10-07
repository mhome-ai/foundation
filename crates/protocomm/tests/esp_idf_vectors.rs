use num_bigint::BigUint;
use protocomm::sec2::Sec2Cipher;
use protocomm::srp::{verifier_for, SrpClient, SrpServer, GROUP_BYTES};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!("vectors/esp-idf-sec2.json")).unwrap()
}

fn bytes(vector: &Value, key: &str) -> Vec<u8> {
    hex::decode(vector[key].as_str().unwrap()).unwrap()
}

fn text<'a>(vector: &'a Value, key: &str) -> &'a str {
    vector[key].as_str().unwrap()
}

fn key(vector: &Value) -> [u8; 64] {
    bytes(vector, "K").try_into().unwrap()
}

fn server(vector: &Value) -> SrpServer {
    SrpServer::with_ephemeral(
        text(vector, "username"),
        &bytes(vector, "salt"),
        &bytes(vector, "verifier"),
        BigUint::from_bytes_be(&bytes(vector, "b")),
    )
}

fn check_device_side(vector: &Value) {
    let device = server(vector);
    assert_eq!(device.public_key(), bytes(vector, "B"));
    let session = device
        .verify_client(&bytes(vector, "A"), &bytes(vector, "M"))
        .unwrap();
    assert_eq!(session.device_proof, bytes(vector, "HAMK"));
    assert_eq!(session.key, key(vector));
}

fn check_client_side(vector: &Value) {
    assert_eq!(
        verifier_for(
            &bytes(vector, "salt"),
            text(vector, "username"),
            text(vector, "password")
        ),
        bytes(vector, "verifier")
    );
    let client = SrpClient::with_ephemeral(
        text(vector, "username"),
        text(vector, "password"),
        BigUint::from_bytes_be(&bytes(vector, "a")),
    )
    .unwrap();
    assert_eq!(client.public_key(), bytes(vector, "A"));
    let proof = client
        .process_challenge(&bytes(vector, "salt"), &bytes(vector, "B"))
        .unwrap();
    assert_eq!(proof.proof, bytes(vector, "M"));
    assert_eq!(proof.verify(&bytes(vector, "HAMK")).unwrap(), key(vector));
}

#[test]
fn matches_esp_idf_for_full_length_values() {
    let vectors = vectors();
    check_client_side(&vectors["basic"]);
    check_device_side(&vectors["basic"]);
}

#[test]
fn hashes_a_leading_zero_salt_as_sent() {
    let vector = &vectors()["leadingZeroSalt"];
    assert_eq!(bytes(vector, "salt")[0], 0);
    check_client_side(vector);
    check_device_side(vector);
}

#[test]
fn keeps_a_short_device_key_unpadded() {
    let vector = &vectors()["leadingZeroB"];
    assert!(bytes(vector, "B").len() < GROUP_BYTES);
    check_client_side(vector);
    check_device_side(vector);
}

#[test]
fn never_sends_a_client_key_with_a_leading_zero() {
    let vector = &vectors()["leadingZeroA"];
    let a = bytes(vector, "A");
    assert_eq!((a.len(), a[0]), (GROUP_BYTES, 0));
    assert!(SrpClient::with_ephemeral(
        text(vector, "username"),
        text(vector, "password"),
        BigUint::from_bytes_be(&bytes(vector, "a")),
    )
    .is_none());
    check_device_side(vector);
}

#[test]
fn records_follow_the_esp_idf_nonce_counter() {
    let vector = &vectors()["basic"];
    let nonce = bytes(vector, "deviceNonce");
    let mut client = Sec2Cipher::new(&key(vector), &nonce, 1).unwrap();
    let mut device = Sec2Cipher::new(&key(vector), &nonce, 1).unwrap();
    for record in vector["records"].as_array().unwrap() {
        let plaintext = text(record, "plaintext").as_bytes();
        let wire = bytes(record, "record");
        let (sender, receiver) = if text(record, "from") == "client" {
            (&mut client, &mut device)
        } else {
            (&mut device, &mut client)
        };
        assert_eq!(sender.encrypt(plaintext).unwrap(), wire);
        assert_eq!(receiver.decrypt(&wire).unwrap(), plaintext);
    }

    let after = &vector["afterFailure"];
    let plaintext = text(after, "plaintext").as_bytes();
    assert_eq!(
        client.encrypt(plaintext).unwrap(),
        bytes(after, "lostRecord")
    );
    let stale = client.encrypt(plaintext).unwrap();
    assert_eq!(stale, bytes(after, "staleRecord"));
    assert!(device.decrypt(&stale).is_err());
    assert_eq!(
        device.decrypt(&bytes(after, "lostRecord")).unwrap(),
        plaintext
    );
}
