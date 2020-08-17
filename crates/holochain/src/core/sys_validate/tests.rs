use super::*;
use ::fixt::prelude::*;
use error::SysValidationError;
use holo_hash::fixt::*;
use holochain_keystore::AgentPubKeyExt;
use holochain_serialized_bytes::{SerializedBytes, UnsafeBytes};
use holochain_types::{
    element::{SignedHeaderHashed, SignedHeaderHashedExt},
    fixt::*,
    test_utils::{fake_agent_pubkey_1, fake_header_hash},
    Timestamp,
};
use holochain_zome_types::{header::InitZomesComplete, Header};
use matches::assert_matches;
use std::convert::TryInto;

async fn test_gen(ts: Timestamp, seq: u32, prev: HeaderHash) -> Element {
    let keystore = holochain_state::test_utils::test_keystore();

    let header = InitZomesComplete {
        author: fake_agent_pubkey_1(),
        timestamp: ts.into(),
        header_seq: seq,
        prev_header: prev,
    };

    let hashed = HeaderHashed::from_content(header.into()).await;
    let signed = SignedHeaderHashed::new(&keystore, hashed).await.unwrap();
    Element::new(signed, None)
}

#[tokio::test(threaded_scheduler)]
async fn valid_headers_validate() {
    let first = test_gen(
        "2020-05-05T19:16:04.266431045Z".try_into().unwrap(),
        12,
        fake_header_hash(1),
    )
    .await;
    let second = test_gen(
        "2020-05-05T19:16:04.366431045Z".try_into().unwrap(),
        13,
        first.header_address().clone(),
    )
    .await;

    sys_validate_element(&fake_agent_pubkey_1(), &second, Some(&first))
        .await
        .unwrap();
}

#[tokio::test(threaded_scheduler)]
async fn invalid_hash_headers_dont_validate() {
    let first = test_gen(
        "2020-05-05T19:16:04.266431045Z".try_into().unwrap(),
        12,
        fake_header_hash(1),
    )
    .await;
    let second = test_gen(
        "2020-05-05T19:16:04.366431045Z".try_into().unwrap(),
        13,
        fake_header_hash(2),
    )
    .await;

    matches::assert_matches!(
        sys_validate_element(&fake_agent_pubkey_1(), &second, Some(&first)).await,
        Err(SourceChainError::InvalidPreviousHeader(_))
    );
}

#[tokio::test(threaded_scheduler)]
async fn invalid_timestamp_headers_dont_validate() {
    let first = test_gen(
        "2020-05-05T19:16:04.266431045Z".try_into().unwrap(),
        12,
        fake_header_hash(1),
    )
    .await;
    let second = test_gen(
        "2020-05-05T19:16:04.166431045Z".try_into().unwrap(),
        13,
        first.header_address().clone(),
    )
    .await;

    matches::assert_matches!(
        sys_validate_element(&fake_agent_pubkey_1(), &second, Some(&first)).await,
        Err(SourceChainError::InvalidPreviousHeader(_))
    );
}

#[tokio::test(threaded_scheduler)]
async fn invalid_seq_headers_dont_validate() {
    let first = test_gen(
        "2020-05-05T19:16:04.266431045Z".try_into().unwrap(),
        12,
        fake_header_hash(1),
    )
    .await;
    let second = test_gen(
        "2020-05-05T19:16:04.366431045Z".try_into().unwrap(),
        14,
        first.header_address().clone(),
    )
    .await;

    matches::assert_matches!(
        sys_validate_element(&fake_agent_pubkey_1(), &second, Some(&first)).await,
        Err(SourceChainError::InvalidPreviousHeader(_))
    );
}

#[tokio::test(threaded_scheduler)]
async fn verify_header_signature_test() {
    let keystore = holochain_state::test_utils::test_keystore();
    let author = fake_agent_pubkey_1();
    let mut header = fixt!(LinkAdd);
    header.author = author.clone();
    let header = Header::LinkAdd(header);
    let real_signature = author.sign(&keystore, &header).await.unwrap();
    let wrong_signature = Signature(vec![1; 64]);

    assert_matches!(
        verify_header_signature(&wrong_signature, &header).await,
        Err(SysValidationError::VerifySignature(_, _))
    );

    assert_matches!(
        verify_header_signature(&real_signature, &header).await,
        Ok(())
    );
}

#[tokio::test(threaded_scheduler)]
async fn check_previous_header() {
    let mut header = fixt!(LinkAdd);
    header.prev_header = fixt!(HeaderHash);
    header.header_seq = 1;
    assert_matches!(check_prev_header(&header.clone().into()), Ok(()));
    header.header_seq = 0;
    assert_matches!(
        check_prev_header(&header.clone().into()),
        Err(SysValidationError::PrevHeaderError(
            PrevHeaderError::InvalidRoot
        ))
    );
    // Dna is always ok because of the type system
    let header = fixt!(Dna);
    assert_matches!(check_prev_header(&header.into()), Ok(()));
}

#[tokio::test(threaded_scheduler)]
async fn check_previous_timestamp() {
    let mut header = fixt!(LinkAdd);
    let mut prev_header = fixt!(LinkAdd);
    header.timestamp = Timestamp::now().into();
    let before = chrono::Utc::now() - chrono::Duration::weeks(1);
    let after = chrono::Utc::now() + chrono::Duration::weeks(1);

    prev_header.timestamp = Timestamp::from(before).into();
    let r = check_prev_timestamp(&header.clone().into(), &prev_header.clone().into());
    assert_matches!(r, Ok(()));

    prev_header.timestamp = Timestamp::from(after).into();
    let r = check_prev_timestamp(&header.clone().into(), &prev_header.clone().into());
    assert_matches!(
        r,
        Err(SysValidationError::PrevHeaderError(
            PrevHeaderError::Timestamp
        ))
    );
}

#[tokio::test(threaded_scheduler)]
async fn check_previous_seq() {
    let mut header = fixt!(LinkAdd);
    let mut prev_header = fixt!(LinkAdd);

    header.header_seq = 2;
    prev_header.header_seq = 1;
    assert_matches!(
        check_prev_seq(&header.clone().into(), &prev_header.clone().into()),
        Ok(())
    );

    prev_header.header_seq = 2;
    assert_matches!(
        check_prev_seq(&header.clone().into(), &prev_header.clone().into()),
        Err(SysValidationError::PrevHeaderError(PrevHeaderError::InvalidSeq(_, _)))
    );

    prev_header.header_seq = 3;
    assert_matches!(
        check_prev_seq(&header.clone().into(), &prev_header.clone().into()),
        Err(SysValidationError::PrevHeaderError(PrevHeaderError::InvalidSeq(_, _)))
    );

    header.header_seq = 0;
    prev_header.header_seq = 0;
    assert_matches!(
        check_prev_seq(&header.clone().into(), &prev_header.clone().into()),
        Err(SysValidationError::PrevHeaderError(PrevHeaderError::InvalidSeq(_, _)))
    );
}

#[tokio::test(threaded_scheduler)]
async fn check_entry_type_test() {
    let entry_fixt = EntryFixturator::new(Predictable);
    let et_fixt = EntryTypeFixturator::new(Predictable);

    for (e, et) in entry_fixt.zip(et_fixt).take(4) {
        assert_matches!(check_entry_type(&et, &e), Ok(()));
    }

    // Offset by 1
    let entry_fixt = EntryFixturator::new(Predictable);
    let mut et_fixt = EntryTypeFixturator::new(Predictable);
    et_fixt.next().unwrap();

    for (e, et) in entry_fixt.zip(et_fixt).take(4) {
        assert_matches!(
            check_entry_type(&et, &e),
            Err(SysValidationError::EntryType)
        );
    }
}

#[tokio::test(threaded_scheduler)]
async fn check_entry_hash_test() {
    let mut ec = fixt!(EntryCreate);
    let entry = fixt!(Entry);
    let hash = EntryHash::with_data(&entry).await;
    let header: Header = ec.clone().into();

    // First check it should have an entry
    assert_matches!(check_new_entry_header(&header), Ok(()));
    // Safe to unwrap if new entry
    let eh = header.entry_data().map(|(h, _)| h).unwrap();
    assert_matches!(
        check_entry_hash(&eh, &entry).await,
        Err(SysValidationError::EntryHash)
    );

    ec.entry_hash = hash;
    let header: Header = ec.clone().into();

    let eh = header.entry_data().map(|(h, _)| h).unwrap();
    assert_matches!(check_entry_hash(&eh, &entry).await, Ok(()));
    assert_matches!(
        check_new_entry_header(&fixt!(LinkAdd).into()),
        Err(SysValidationError::NotNewEntry(_))
    );
}

#[tokio::test(threaded_scheduler)]
async fn check_entry_size_test() {
    let tiny = Entry::App(SerializedBytes::from(UnsafeBytes::from(vec![0; 1])));
    let bytes = (0..16_000_000).map(|_| 0u8).into_iter().collect::<Vec<_>>();
    let huge = Entry::App(SerializedBytes::from(UnsafeBytes::from(bytes)));
    assert_matches!(check_entry_size(&tiny), Ok(()));

    assert_matches!(
        check_entry_size(&huge),
        Err(SysValidationError::EntryTooLarge(_, _))
    );
}