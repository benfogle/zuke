use zuke::{given, Context};

#[derive(Debug, Eq, PartialEq)]
enum Color {
    Red,
    Green,
    Blue,
}

impl std::str::FromStr for Color {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "red" => Self::Red,
            "green" => Self::Green,
            "blue" => Self::Blue,
            _ => anyhow::bail!("invalid color"),
        })
    }
}

#[given("a step that captures context")]
async fn cap_context(context: &mut Context) {
    drop(context);
}

#[given("a step that captures _context")]
async fn cap_context_unused(_context: &mut Context) {}

#[given(regex, r#"a regex step that expects "(?P<what>.*)""#)]
async fn expects_foo(what: String) {
    assert_eq!(what, "foo");
}

#[given(
    regex,
    r#"a regex step that captures context and expects "(?P<what>.*)""#
)]
async fn expects_context_foo(_context: &mut Context, what: String) {
    assert_eq!(what, "foo");
}

#[given(regex, r#"a regex step that expects number (?P<num>\d+)"#)]
async fn expects_hundred(num: u32) {
    assert_eq!(num, 100)
}

#[given(regex, r#"a regex step that expects str "(?P<what>.*)""#)]
async fn expects_foo_str(what: &str) {
    assert_eq!(what, "foo")
}

#[given(regex, r#"a regex step that expects the color (?P<color>.*)"#)]
async fn expects_color_red(color: Color) {
    assert_eq!(color, Color::Red);
}

#[given(
    regex,
    r#"a regex step that captures "(?P<context>.*)" using the name context"#
)]
async fn expects_foo_context(context: &str) {
    assert_eq!(context, "foo")
}

#[given(
    regex,
    r#"a regex step that captures "(?P<_context>.*)" using the name _context"#
)]
async fn expects_foo_context_unused(_context: &str) {
    assert_eq!(_context, "foo")
}

#[given("a step that expects \"{what}\"")]
async fn expects_foo_basic(what: String) {
    assert_eq!(what, "foo");
}

#[given("a step that captures context and expects \"{what}\"")]
async fn expects_context_foo_basic(_context: &mut Context, what: String) {
    assert_eq!(what, "foo");
}

#[given(r#"a step that expects number {num:\d+}"#)]
async fn expects_hundred_basic(num: u32) {
    assert_eq!(num, 100)
}

#[given("a step that expects str \"{what}\"")]
async fn expects_foo_str_basic(what: &str) {
    assert_eq!(what, "foo")
}

#[given("a step that expects the color {color}")]
async fn expects_color_red_basic(color: Color) {
    assert_eq!(color, Color::Red);
}

#[given("a step that captures \"{context}\" using the name context")]
async fn expects_foo_context_basic(context: &str) {
    assert_eq!(context, "foo")
}

#[given("a step that captures \"{_context}\" using the name _context")]
async fn expects_foo_context_unused_basic(_context: &str) {
    assert_eq!(_context, "foo")
}
