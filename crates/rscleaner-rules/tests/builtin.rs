use std::collections::BTreeSet;

use rscleaner_model::Platform;

#[test]
fn builtin_rules_are_valid_and_unique() {
    let set = match rscleaner_rules::builtin::load_builtin() {
        Ok(set) => set,
        Err(errors) => panic!("built-in rules are invalid: {errors:?}"),
    };

    assert_eq!(
        set.rules
            .values()
            .filter(|rule| rule.platforms.contains(&Platform::Windows))
            .count(),
        32
    );
    assert!(
        set.rules
            .values()
            .any(|rule| rule.platforms.contains(&Platform::Linux))
    );

    let mut ids = BTreeSet::new();
    for rule in set.rules.values() {
        assert!(ids.insert(rule.id.clone()), "duplicate {}", rule.id);
        assert!(!rule.targets.is_empty(), "{} has no targets", rule.id);
    }
}
