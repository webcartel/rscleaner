use rscleaner_i18n::{Language, Translator};

#[test]
fn missing_russian_key_falls_back_to_english() {
    let translator = Translator::with_overrides(Language::Russian, [("only.en", "English")]);
    assert_eq!(translator.t("only.en"), "English");
}

#[test]
fn russian_plural_uses_three_forms() {
    assert_eq!(rscleaner_i18n::plural_ru(1), 0);
    assert_eq!(rscleaner_i18n::plural_ru(3), 1);
    assert_eq!(rscleaner_i18n::plural_ru(11), 2);
}
