use shufti_macro::ShuftiMatcher;
use shufti_matcher::ShuftiMatch;

#[derive(ShuftiMatcher)]
#[shufti(set = "\t\r\n")]
pub struct WsMatcher;

#[derive(ShuftiMatcher)]
#[shufti(set = "\0\t\n\r #/:<>?@[\\]^|")]
pub struct MyMatcher;

fn main() {
    let s = b"\nhello\nworld";
    let pos = WsMatcher::find_first(s);
    assert_eq!(pos, Some(0));
    let pos = WsMatcher::find_first(&s[1..]);
    assert_eq!(pos, Some(5));

    let s1 = b"\nhello";
    assert_eq!(MyMatcher::find_first(s1), Some(0));

    let s2 = b"user@domain";
    assert_eq!(MyMatcher::find_first(s2), Some(4));

    let s3 = b"path\\to";
    assert_eq!(MyMatcher::find_first(s3), Some(4));

    let s4 = b"rust|cpp";
    assert_eq!(MyMatcher::find_first(s4), Some(4));

    let s5 = b"ABCdef123";
    assert_eq!(MyMatcher::find_first(s5), None);

    let s6 = b"null\0byte";
    assert_eq!(MyMatcher::find_first(s6), Some(4));

    let s7 = b" \t ";
    assert_eq!(MyMatcher::find_first(s7), Some(0));
    assert_eq!(MyMatcher::find_first(&s7[1..]), Some(0));
}
