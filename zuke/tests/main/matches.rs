use zuke::given;

#[given(regex, "a regex step that returns nothing")]
#[given("a step with special characters...")]
#[given(regex, r#"a word with a double vowel ".*(aa|ee|ii|oo|uu).*""#)]
fn do_nothing() {}
