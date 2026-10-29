Similar to shell glob, but without `?` and `[]` meta character, and `text` is 'contains' instead of 'exact' matching

| contains pattern  | behavior                                                  |
| ---               | ---                                                       |
| `text`            | `s.contains("text")`                                      |
| `prefix*`         | `s.starts_with("prefix")`                                 |
| `*suffix`         | `s.ends_with("suffix")`                                   |
| `prefix*suffix`   | `s.starts_with("prefix") && s.ends_with("suffix")`        |
| `*exact*`         | `s == "exact"`                                            |

Except for `***` `****`, which can match `*` `**` etc, this library does not have the ability to match `*`

This syntax is between regexp and glob, with predictable matching time, and is suitable for some command-line filters

| contains pattern  | shell glob        | regexp            |
| ---               | ---               | ---               |
| `text`            | `*text*`          | `text`            |
| `prefix*`         | `prefix*`         | `^prefix`         |
| `*suffix`         | `*suffix`         | `suffix$`         |
| `prefix*suffix`   | `prefix*suffix`   | `^prefix.*suffix$`|
| `*exact*`         | `exact`           | `^exact$`         |

# Motivation
On the cli, 'contains' logic is often used for filtering, such as 'test modules'
They often use simplified regexps to match (not supported by `(?=...)`)

I think that at the cost of some inconsistency (`*exact*`),
delegating some syntax to cli-args (use shell brace expansion `-Ifoo_{bar,baz}` instead of `-I'foo_(bar|baz)'`) can result in very practical matches

# Side case

| contains pattern  | behavior          |
| ---               | ---               |
| `*`               | `s.contains("*")` |
| `**`              | `s == ""`         |
| `***`             | `s == "*"`        |
| empty             | `true`            |

# Examples

```rust
use contains_match::Pattern;

let pattern = Pattern::new("foo*").unwrap();
assert_eq!(pattern.is_match("foobar"), true);
assert_eq!(pattern.is_match("barfoo"), false);

let pattern = Pattern::new("foo").unwrap(); // contains logic when without '*'
assert_eq!(pattern.is_match("foobar"), true);
assert_eq!(pattern.is_match("barfoo"), true);

let pattern = Pattern::new("*foo*").unwrap(); // exact match logic for '*...*'
assert_eq!(pattern.is_match("foo"),    true);
assert_eq!(pattern.is_match("foobar"), false);
assert_eq!(pattern.is_match("barfoo"), false);
```
