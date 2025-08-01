//! Tests various forms of raw string literals

//@ run-pass

// ignore-tidy-tab: Raw strings deliberately contain tabs
// ignore-tidy-linelength: Long pattern string for vim syntax test

pub fn main() {
    assert_eq!(r"abc", "abc");

    assert_eq!(r#"abc"#, "abc");

    assert_eq!(r"###", "###");

    assert_eq!(r"\", "\\");

    assert_eq!(r#"\""#, "\\\"");

    assert_eq!(r#"#"\n""#, "#\"\\n\"");

    assert_eq!(r##"a"#"b"##, "a\"#\"b");

    // from rust.vim
    assert_eq!(
        r#""%\(\d\+\$\)\=[-+' #0*]*\(\d*\|\*\|\*\d\+\$\)\(\.\(\d*\|\*\|\*\d\+\$\)\)\=\([hlLjzt]\|ll\|hh\)\=\([aAbdiuoxXDOUfFeEgGcCsSpn?]\|\[\^\=.[^]]*\]\)""#,
        "\"%\\(\\d\\+\\$\\)\\=[-+' #0*]*\\(\\d*\\|\\*\\|\\*\\d\\+\\$\\)\\(\\.\\(\\d*\\|\\*\\|\\*\\d\\+\\$\\)\\)\\=\\([hlLjzt]\\|ll\\|hh\\)\\=\\([aAbdiuoxXDOUfFeEgGcCsSpn?]\\|\\[\\^\\=.[^]]*\\]\\)\""
    );

    assert_eq!(
        r"newline:'
', tab:'	', unicode:'●', null:' '",
        "newline:'\n', tab:'\t', unicode:'\u{25cf}', null:'\0'"
    );
}
