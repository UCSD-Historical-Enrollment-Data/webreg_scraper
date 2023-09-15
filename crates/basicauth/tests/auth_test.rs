use basicauth::{AuthCheckResult, AuthManager};

const MEMORY_DB: &str = ":memory:";
#[test]
fn test_add_keys_check() {
    let manager = AuthManager::new(MEMORY_DB);
    let key1 = manager.generate_api_key(Some("this is a test"));
    let key2 = manager.generate_api_key(Some("this is another test"));

    let (prefix1, token1) = key1.split_once('#').unwrap();
    let (prefix2, token2) = key2.split_once('#').unwrap();

    assert_eq!(AuthCheckResult::Valid, manager.check_key(prefix1, token1));
    assert_eq!(AuthCheckResult::Valid, manager.check_key(prefix2, token2));
    assert_eq!(
        AuthCheckResult::NoPrefixOrKeyFound,
        manager.check_key(prefix2, token1)
    );
    assert_eq!(
        AuthCheckResult::NoPrefixOrKeyFound,
        manager.check_key(prefix1, token2)
    );
}

#[test]
fn test_get_all_prefixes() {
    let manager = AuthManager::new(MEMORY_DB);
    let key1 = manager.generate_api_key(Some("this is a test"));
    let key2 = manager.generate_api_key(Some("this is another test"));

    let (prefix1, _) = key1.split_once('#').unwrap();
    let (prefix2, _) = key2.split_once('#').unwrap();

    let expected = vec![prefix1.to_owned(), prefix2.to_owned()];
    assert_eq!(expected, manager.get_all_prefixes());
}

#[test]
fn test_edit_description() {
    let manager = AuthManager::new(MEMORY_DB);
    let key1 = manager.generate_api_key(Some("this is a test"));
    manager.generate_api_key(Some("this is another test"));
    let (prefix1, _) = key1.split_once('#').unwrap();

    let all_entries = manager.get_all_entries();
    assert_eq!(
        Some("this is a test".to_owned()),
        all_entries[0].description
    );
    assert_eq!(
        Some("this is another test".to_owned()),
        all_entries[1].description
    );

    manager.edit_description_by_prefix(prefix1, Some("this is a test 2.0"));
    let all_entries2 = manager.get_all_entries();
    assert_eq!(
        Some("this is a test 2.0".to_owned()),
        all_entries2[0].description
    );
    assert_eq!(
        Some("this is another test".to_owned()),
        all_entries2[1].description
    );
}

#[test]
fn test_delete_key() {
    let manager = AuthManager::new(MEMORY_DB);
    manager.generate_api_key(Some("this is a test"));
    let key2 = manager.generate_api_key(Some("this is another test"));
    manager.generate_api_key(Some("this is a third test"));
    let (prefix2, token2) = key2.split_once('#').unwrap();

    let all_prefixes = manager.get_all_prefixes();
    assert_eq!(3, all_prefixes.len());
    assert!(manager.delete_by_prefix(prefix2));
    assert_eq!(
        AuthCheckResult::NoPrefixOrKeyFound,
        manager.check_key(prefix2, token2)
    );

    let all_prefixes2 = manager.get_all_prefixes();
    assert_eq!(2, all_prefixes2.len());
    assert!(!manager.delete_by_prefix(prefix2));
}
