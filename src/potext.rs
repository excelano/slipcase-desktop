//! Message catalogs in the GNU `.po` format, compiled into the binary as text.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)
//
// **This module is `potext` before it is a crate.** It is written to be lifted
// out whole: nothing here knows what application it is in, and the only thing
// that will change on extraction is the `mod` line becoming a dependency. The
// second consumer is `excelano/flyleaf`, whose metadata tree draws inside this
// window and carries its own strings, and a published crate cannot reach into
// this one's `src/`. That is the whole reason it becomes a crate rather than
// staying a module.
//
// It lives in the library rather than beside the window because this package
// is two crates: `size_line` and the messages around it are in `src/lib.rs`,
// and a module declared in `src/main.rs` is in the other crate and out of
// their reach.
//
// **Nothing is translated until `main` says so.** `activate` is called there
// and nowhere else, so `cargo test`, the golden files and the conformance
// runner all see the English their assertions are written in, whatever
// language the machine running them is set to. A catalogue loaded from an
// ambient `LANG` would make the test suite pass or fail by locale, which is a
// defect this design does not have rather than one it defends against.
//
// **Why `.po` rather than a Rust i18n crate.** Three things were weighed on
// 2026-09-09. `gettext-rs` is out on this tree's own rule: `gettext-sys`
// compiles C. `rust-i18n` works, is pure Rust, and would have cost no code at
// all — what it does not have is a fuzzy state. When an English string is
// reworded, gettext's `msgmerge` pairs the new text with the old entry by
// similarity, attaches the previous translation, and marks it `#, fuzzy`;
// a runtime that honours the mark then shows *English* rather than a German
// sentence that no longer says what the English says. A key-value scanner
// cannot do that: the reworded string is simply a new key, and the old
// translation sits in the file still looking finished. Across six applications
// and a few hundred strings that difference is the whole maintenance story, so
// the format that carries the mark won and this module is the runtime that
// honours it. Comma already speaks `.po`, so the fleet ends with one format,
// one glossary and one `msgfmt --statistics` check.
//
// **What this is not.** It is not gettext. There is no `.mo`, no text domain,
// no `bindtextdomain`, and no catalogue on disk to install or find — which is
// what makes it work identically in an App Store sandbox, an MSIX package and
// a `.deb`, none of which agree about where a data file lives. The catalogue is
// `include_str!`ed by the caller and parsed once at startup.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The separator between a context and its message id in a keyed lookup.
///
/// EOT, which is what gettext's own `.mo` format puts between the two, so a
/// catalogue that ever passes through `msgfmt` keys the same way this does.
const CONTEXT_SEPARATOR: char = '\u{4}';

/// The environment variable that overrides what the platform says.
///
/// Deliberately named for the crate rather than the application: one variable
/// then drives every application in the fleet, which is what makes a
/// pseudolocale worth having. `LANGUAGE`, gettext's own override, is honoured
/// after it so that a machine already set up for translated software behaves
/// the way its owner expects.
const OVERRIDE: &str = "POTEXT_LANG";

/// A message, in the one form or the two a plural needs.
enum Message {
    Single(String),
    /// Indexed by the plural rule's answer, so `[0]` is the singular.
    Plural(Vec<String>),
}

/// One language's messages, parsed from a `.po` file's text.
pub struct Catalog {
    /// Messages with no context, keyed by their English text.
    ///
    /// Separate from the keyed map below so the common lookup can be made with
    /// a `&str` and allocate nothing. An immediate-mode window asks for every
    /// visible string on every frame, so a `String` built per lookup would be a
    /// few hundred allocations sixty times a second for no reader's benefit.
    plain: HashMap<String, Message>,
    /// Messages with a `msgctxt`, keyed by context, EOT, then English text.
    keyed: HashMap<String, Message>,
    /// Whether the header's `Plural-Forms` is the two-form rule this understands.
    ///
    /// English and German share it, and it is the only rule here: see `tn`.
    two_forms: bool,
}

/// The catalogue in force, or nothing, which means English.
///
/// A `static` rather than a field on the application so that `t` can be called
/// from anywhere without threading a reference through every function that
/// draws. It is written once, before the window exists, and read on every
/// frame afterwards — and because the `OnceLock` is itself `static`, a
/// reference into it is `&'static`, which is what lets a lookup hand back a
/// `&'static str` rather than a fresh `String`.
static CATALOG: OnceLock<Catalog> = OnceLock::new();

/// This message in the language in force, or the English it was written in.
///
/// The English text is the key, so a call site reads as the sentence a person
/// sees and an untranslated string is not a missing-key placeholder but the
/// original. Takes `&'static str` and returns one: every call site is a
/// literal, and the catalogue outlives the program.
#[must_use]
pub fn t(msgid: &'static str) -> &'static str {
    match CATALOG.get().and_then(|c| c.plain.get(msgid)) {
        Some(Message::Single(translated)) => translated,
        // A plural entry reached through `t` is a catalogue that disagrees with
        // its call site. English is the safe answer: it is at least the text
        // the programmer wrote.
        Some(Message::Plural(_)) | None => msgid,
    }
}

/// This message in the language in force, where the English is ambiguous.
///
/// The context is not shown to anybody; it exists so that one English word
/// standing for two different things can be two entries in the catalogue.
/// German needs this more often than English suggests — *open* the verb and
/// *open* the adjective are `öffnen` and `offen`.
#[must_use]
pub fn tc(context: &'static str, msgid: &'static str) -> &'static str {
    let Some(catalog) = CATALOG.get() else {
        return msgid;
    };
    let mut key = String::with_capacity(context.len() + 1 + msgid.len());
    key.push_str(context);
    key.push(CONTEXT_SEPARATOR);
    key.push_str(msgid);
    match catalog.keyed.get(&key) {
        Some(Message::Single(translated)) => translated,
        Some(Message::Plural(_)) | None => msgid,
    }
}

/// One of two English forms, or the translated form the count calls for.
///
/// **The only plural rule understood is two forms chosen by `n != 1`**, which
/// is what English and German both use and what `msginit` writes for German.
/// A catalogue declaring anything else keeps its singular and plural entries
/// but is not consulted here, and the English forms are used instead — wrong
/// language, right grammar, which is the better of the two ways to be wrong.
/// A language with three forms is the day this grows a rule evaluator, and
/// nothing in the fleet needs one today.
#[must_use]
pub fn tn(one: &'static str, other: &'static str, n: u64) -> &'static str {
    let english = if n == 1 { one } else { other };
    let Some(catalog) = CATALOG.get() else {
        return english;
    };
    if !catalog.two_forms {
        return english;
    }
    match catalog.plain.get(one) {
        Some(Message::Plural(forms)) => {
            let wanted = usize::from(n != 1);
            forms.get(wanted).map_or(english, String::as_str)
        }
        Some(Message::Single(_)) | None => english,
    }
}

/// Put the values into a translated message's placeholders.
///
/// A translated sentence cannot take Rust's inline `{name}` capture, because
/// the text a formatting macro is given has to be a literal and the whole point
/// here is that it is not. This is the substitution instead, and Comma spells
/// it `gettext(...).replace(...)` for the same reason.
///
/// **It is one pass, and that is the reason it exists rather than a chain of
/// `replace` calls.** Half these messages carry a payload's name or a path,
/// both of which come from whoever made the container, and a chain would go
/// looking for the second placeholder *inside the name the first one just
/// inserted*. A file called `{reason}.pdf` would rewrite the sentence around
/// it. Scanning the template once and never rescanning what was substituted is
/// what makes that impossible rather than unlikely.
///
/// A placeholder with no value keeps its braces, so a translator who invents
/// one sees it in the window rather than losing it.
#[must_use]
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            // An unclosed brace is the rest of the message, kept as it is.
            out.push('{');
            out.push_str(after);
            return out;
        };
        let name = &after[..close];
        if let Some((_, value)) = values.iter().find(|(key, _)| *key == name) {
            out.push_str(value);
        } else {
            out.push('{');
            out.push_str(name);
            out.push('}');
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

/// Read the platform's language, pick a catalogue for it, and put it in force.
///
/// Each pair is a language tag and that language's `.po` file as text, which
/// the caller will have `include_str!`ed. Returns the tag chosen, for a test
/// or a log to assert on; `None` means nothing matched and the interface stays
/// in the English its call sites are written in.
///
/// Calling it twice is not an error and the second call does nothing, because
/// the window reads the catalogue on every frame and swapping it underneath
/// would mean locking on a read that has no other reason to lock. A language
/// change is a restart, which is what a desktop application's language setting
/// is everywhere else.
pub fn activate(available: &[(&str, &'static str)]) -> Option<String> {
    let preferred = preferred()?;
    let (tag, text) = choose(available, &preferred)?;
    let tag = tag.to_owned();
    let _ = CATALOG.set(Catalog::parse(text));
    Some(tag)
}

/// The catalogue whose language best answers what the platform asked for.
///
/// Exact tag first, then the language alone, so a machine set to `de-AT` gets
/// the `de` catalogue rather than English, while an `en-GB` catalogue would
/// still be preferred over `en` for a machine that asked for it by name.
fn choose<'a>(
    available: &[(&'a str, &'static str)],
    preferred: &str,
) -> Option<(&'a str, &'static str)> {
    let wanted = normalise(preferred)?;
    let language = wanted.split('-').next().unwrap_or(&wanted).to_owned();
    let matching = |want: &str| {
        available
            .iter()
            .find(|(tag, _)| normalise(tag).as_deref() == Some(want))
            .copied()
    };
    matching(&wanted).or_else(|| matching(&language))
}

/// The language the platform says this person reads, in lower case.
///
/// The environment comes first on every platform, and not only on the two that
/// set it: it is how a pseudolocale is loaded, how a translator checks their
/// work, and how somebody whose desktop is in one language runs one
/// application in another.
#[must_use]
pub fn preferred() -> Option<String> {
    // POSIX order, and the first one that is set decides — including when what
    // it says is `C`, which means *no locale* and must not fall through to a
    // `LANG` left over beneath it.
    for name in [OVERRIDE, "LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Some(value) = std::env::var_os(name) {
            let value = value.to_string_lossy().into_owned();
            if !value.is_empty() {
                // `LANGUAGE` is a colon-separated list in preference order.
                // Only the first is honoured here; a full fallback chain would
                // need `activate` to take one, and no application in the fleet
                // ships more than two languages.
                let first = value.split(':').next().unwrap_or(&value);
                return normalise(first);
            }
        }
    }
    platform()
}

/// What the desktop environment says, where it does not say it in the environment.
///
/// Linux sets `LANG` for everything, graphical or not, so there is nothing
/// left to ask by the time this is reached.
#[cfg(target_os = "linux")]
fn platform() -> Option<String> {
    None
}

/// What macOS says, which is never in the environment for a launched application.
///
/// `LANG` is set by Terminal and by nothing else, so a `.app` opened from
/// Finder or the Dock — which is every way a person actually opens this — has
/// no environment to read and this is the only answer available.
///
/// `preferredLanguages` is the ordered list from System Settings and its first
/// entry is what other applications follow. It is a safe function: the unsafe
/// belongs to `objc2-foundation`, as `rfd`'s and `opener`'s do, and this module
/// compiles under the crate's `deny(unsafe_code)` with no `allow` beneath it.
///
/// **Written on Linux and never run.** `cargo check --target
/// aarch64-apple-darwin` says it compiles and says nothing about what it
/// returns. `CHECKLIST.md` carries the item for the session that has a Mac.
#[cfg(target_os = "macos")]
fn platform() -> Option<String> {
    let languages = objc2_foundation::NSLocale::preferredLanguages();
    let first = languages.firstObject()?;
    normalise(&first.to_string())
}

/// What Windows says, which is in the registry rather than the environment.
///
/// `LocaleName` under `Control Panel\International` is the user's locale as a
/// BCP-47 tag — `de-DE`, `en-US` — and it is the same value
/// `GetUserDefaultLocaleName` returns. Read through `windows-registry`, which
/// is already in this tree for `opens_with`, rather than through an FFI call
/// this crate cannot make under `deny(unsafe_code)`.
///
/// **Written on Linux and never run**, the same caveat as the macOS arm above,
/// and with the same `CHECKLIST.md` item.
#[cfg(target_os = "windows")]
fn platform() -> Option<String> {
    let read = windows_registry::CURRENT_USER
        .open("Control Panel\\International")
        .ok()?
        .get_string("LocaleName")
        .ok()?;
    normalise(&read)
}

/// Anything else compiles and reads only the environment.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform() -> Option<String> {
    None
}

/// A locale name in any of the spellings the three platforms use, as a tag.
///
/// `de_DE.UTF-8` and `de-DE` and `de` all normalise to something a catalogue
/// tag can be compared against. The codeset after a dot and the modifier after
/// an at-sign are both dropped, which loses the script in a name like
/// `sr@latin` — no catalogue in this fleet distinguishes one, and the day one
/// does is the day this grows a case for it.
///
/// `C` and `POSIX` are not languages. They mean the program should behave as
/// it was written, which here is English, so they give nothing rather than a
/// tag that would never match a catalogue anyway.
fn normalise(tag: &str) -> Option<String> {
    let without_codeset = tag.split(['.', '@']).next().unwrap_or(tag);
    let normalised = without_codeset.trim().replace('_', "-").to_lowercase();
    if normalised.is_empty() || normalised == "c" || normalised == "posix" {
        return None;
    }
    Some(normalised)
}

/// Which field a continuation line belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    None,
    Context,
    Id,
    IdPlural,
    Str(usize),
}

/// One entry, as it is read line by line.
#[derive(Default)]
struct Entry {
    fuzzy: bool,
    obsolete: bool,
    context: Option<String>,
    id: Option<String>,
    id_plural: Option<String>,
    strings: Vec<(usize, String)>,
}

impl Catalog {
    /// Read a `.po` file's text.
    ///
    /// **Nothing here fails.** A catalogue is compiled into the binary and
    /// arrives at run time already decided, so there is nobody to report a
    /// syntax error to and no way to act on one: a stray quote in a German
    /// sentence must not be the reason an application will not start. What
    /// cannot be read is left out, which shows the English, which is exactly
    /// what a missing translation does. Syntax is caught where it can be
    /// acted on, by `msgfmt --check` in `po/update-po.sh`, before a commit.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut catalog = Self {
            plain: HashMap::new(),
            keyed: HashMap::new(),
            two_forms: false,
        };
        let mut entry = Entry::default();
        let mut field = Field::None;

        for line in text.lines() {
            let line = line.trim();

            if line.is_empty() {
                catalog.take(std::mem::take(&mut entry));
                field = Field::None;
                continue;
            }

            if let Some(rest) = line.strip_prefix('#') {
                // `#~` marks an entry `msgmerge` has retired: its English is
                // gone from the source, so showing its German would be showing
                // a translation of something nobody can see.
                if rest.starts_with('~') {
                    entry.obsolete = true;
                } else if let Some(flags) = rest.strip_prefix(',') {
                    // The mark this whole module exists to honour. A fuzzy
                    // entry is `msgmerge`'s guess, carried over from a string
                    // whose English has since changed, and showing it would be
                    // showing German that no longer says what the English says.
                    entry.fuzzy |= flags.split(',').any(|flag| flag.trim() == "fuzzy");
                }
                continue;
            }

            if line.starts_with('"') {
                // A continuation: `.po` writes a long message as adjacent
                // quoted lines that join with nothing between them, the
                // newlines being written as `\n` inside the quotes.
                let text = unquote(line);
                match field {
                    Field::Context => push(&mut entry.context, &text),
                    Field::Id => push(&mut entry.id, &text),
                    Field::IdPlural => push(&mut entry.id_plural, &text),
                    Field::Str(index) => {
                        if let Some(slot) = entry.strings.iter_mut().find(|(i, _)| *i == index) {
                            slot.1.push_str(&text);
                        }
                    }
                    Field::None => {}
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix("msgctxt ") {
                entry.context = Some(unquote(rest));
                field = Field::Context;
            } else if let Some(rest) = line.strip_prefix("msgid_plural ") {
                entry.id_plural = Some(unquote(rest));
                field = Field::IdPlural;
            } else if let Some(rest) = line.strip_prefix("msgid ") {
                entry.id = Some(unquote(rest));
                field = Field::Id;
            } else if let Some(rest) = line.strip_prefix("msgstr[") {
                let (index, rest) = match rest.split_once(']') {
                    Some((digits, rest)) => (digits.trim().parse().unwrap_or(0), rest),
                    None => (0, rest),
                };
                entry.strings.push((index, unquote(rest)));
                field = Field::Str(index);
            } else if let Some(rest) = line.strip_prefix("msgstr ") {
                entry.strings.push((0, unquote(rest)));
                field = Field::Str(0);
            }
        }
        catalog.take(entry);
        catalog
    }

    /// File one finished entry, or drop it.
    fn take(&mut self, entry: Entry) {
        let Some(id) = entry.id else {
            return;
        };
        if entry.obsolete {
            return;
        }

        let mut strings = entry.strings;
        strings.sort_by_key(|(index, _)| *index);

        // The header is the entry with an empty id, and its `msgstr` is a block
        // of `Name: value` lines rather than a translation of anything.
        //
        // **It is read before the fuzzy check and not after**, which is the
        // opposite of what this did until the pseudolocale caught it on its
        // first run: `msgen` and `msginit` both mark a generated header
        // `#, fuzzy`, and `msgmerge` can mark one again. Dropping it with the
        // rest left `two_forms` false, so `tn` fell back to the English forms
        // and the size line read `584 bytes` in a window where every other
        // string was translated. A fuzzy mark on the header means the *header*
        // is a template somebody should fill in; it says nothing about whether
        // the plural rule on it is true. `msgfmt` reads it the same way.
        if id.is_empty() && entry.context.is_none() {
            if let Some((_, header)) = strings.first() {
                self.two_forms = two_form_rule(header);
            }
            return;
        }

        if entry.fuzzy {
            return;
        }

        // An empty translation is what an untranslated entry looks like, and it
        // has to stay out of the map: a hit returning "" would draw a blank
        // label where a fallback to the English draws the sentence.
        if strings.iter().any(|(_, text)| text.is_empty()) {
            return;
        }

        let message = if entry.id_plural.is_some() {
            if strings.len() < 2 {
                return;
            }
            Message::Plural(strings.into_iter().map(|(_, text)| text).collect())
        } else {
            match strings.into_iter().next() {
                Some((_, text)) => Message::Single(text),
                None => return,
            }
        };

        match entry.context {
            Some(context) => {
                self.keyed
                    .insert(format!("{context}{CONTEXT_SEPARATOR}{id}"), message);
            }
            None => {
                self.plain.insert(id, message);
            }
        }
    }
}

/// Whether a header declares the two-form rule `tn` implements.
///
/// Compared with the whitespace taken out, because `msginit`, Poedit and a
/// hand-written header space it differently and all three mean the same rule.
fn two_form_rule(header: &str) -> bool {
    header
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Plural-Forms:"))
        .any(|rule| {
            let tight: String = rule.chars().filter(|c| !c.is_whitespace()).collect();
            tight.starts_with("nplurals=2;plural=(n!=1)") || tight.starts_with("nplurals=2;plural=n!=1")
        })
}

/// Append a continuation line to a field that has already been opened.
fn push(field: &mut Option<String>, text: &str) {
    if let Some(existing) = field.as_mut() {
        existing.push_str(text);
    }
}

/// The text inside a `.po` line's quotes, with its escapes read.
///
/// Only the escapes `xgettext` actually writes are decoded. Anything else
/// keeps its backslash, so an unrecognised sequence reaches the window looking
/// wrong rather than silently losing a character.
fn unquote(line: &str) -> String {
    let inside = match (line.find('"'), line.rfind('"')) {
        (Some(first), Some(last)) if last > first => &line[first + 1..last],
        _ => return String::new(),
    };

    let mut out = String::with_capacity(inside.len());
    let mut chars = inside.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('"') => out.push('"'),
            // A trailing backslash is the same one character as an escaped
            // one: there is nothing after it to escape.
            Some('\\') | None => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{choose, fill, normalise, two_form_rule, unquote, Catalog, Message};

    /// A catalogue with one plain entry, used where the test is about lookup
    /// rather than about parsing.
    fn catalog(text: &str) -> Catalog {
        Catalog::parse(text)
    }

    fn single<'a>(catalog: &'a Catalog, id: &str) -> Option<&'a str> {
        match catalog.plain.get(id) {
            Some(Message::Single(text)) => Some(text),
            _ => None,
        }
    }

    /// Would catch the defect this whole module was chosen for: a translation
    /// `msgmerge` marked as a guess being shown as though it were finished.
    ///
    /// The English of a fuzzy entry has changed since the German was written,
    /// so the German is a sentence about something else. English is the right
    /// answer and absence from the map is how it is produced.
    #[test]
    fn a_fuzzy_entry_is_not_loaded_so_the_english_is_shown() {
        let catalog = catalog(
            "#, fuzzy\nmsgid \"Save\"\nmsgstr \"Sichern\"\n\nmsgid \"Undo\"\nmsgstr \"Rückgängig\"\n",
        );
        assert_eq!(single(&catalog, "Save"), None, "the guess must not be loaded");
        assert_eq!(single(&catalog, "Undo"), Some("Rückgängig"));
    }

    /// Would catch a fuzzy mark being missed because it was written beside
    /// other flags, which is how `msgmerge` writes it for a format string.
    #[test]
    fn fuzzy_is_found_among_other_flags() {
        let catalog = catalog("#, fuzzy, rust-format\nmsgid \"Not saved: {e}\"\nmsgstr \"x\"\n");
        assert_eq!(single(&catalog, "Not saved: {e}"), None);
    }

    /// Would catch an entry `msgmerge` retired coming back to life.
    ///
    /// `#~` means the English is gone from the source; the German under it is
    /// a translation of a sentence nobody can see any more.
    #[test]
    fn an_obsolete_entry_is_not_loaded() {
        let catalog = catalog("#~ msgid \"Gone\"\n#~ msgstr \"Weg\"\n");
        assert_eq!(single(&catalog, "Gone"), None);
    }

    /// Would catch an untranslated entry being loaded as an empty string,
    /// which draws a blank label where falling back to English draws the
    /// sentence. Every `.pot` is nothing but entries of this shape.
    #[test]
    fn an_empty_translation_is_absent_rather_than_empty() {
        let catalog = catalog("msgid \"Save\"\nmsgstr \"\"\n");
        assert_eq!(single(&catalog, "Save"), None);
    }

    /// Would catch a fuzzy mark on the header taking the plural rule down with
    /// it, which is what the pseudolocale found on its first run.
    ///
    /// `msgen` and `msginit` both write `#, fuzzy` above a header they
    /// generated, so this is the ordinary state of a new catalogue rather than
    /// an edge case. With the rule lost, `tn` falls back to the English forms
    /// and the one plural in the application is the one string in the window
    /// that stays in English.
    #[test]
    fn a_fuzzy_header_still_declares_the_plural_rule() {
        let catalog = catalog(
            "#, fuzzy\n\
             msgid \"\"\n\
             msgstr \"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\
             \n\
             msgid \"{n} byte\"\n\
             msgid_plural \"{n} bytes\"\n\
             msgstr[0] \"{n} Byte\"\n\
             msgstr[1] \"{n} Bytes\"\n",
        );
        assert!(catalog.two_forms, "the rule is the header's, not the mark's");
        assert!(matches!(
            catalog.plain.get("{n} byte"),
            Some(Message::Plural(_))
        ));
    }

    /// Would catch the header being loaded as a message.
    ///
    /// Its id is empty and its `msgstr` is the `Name: value` block; stored as a
    /// translation it would answer a lookup for the empty string with the whole
    /// header.
    #[test]
    fn the_header_is_not_a_message() {
        let catalog = catalog("msgid \"\"\nmsgstr \"Language: de\\n\"\n");
        assert!(catalog.plain.is_empty());
    }

    /// Would catch continuation lines being joined with anything between them.
    ///
    /// `.po` splits a long message across adjacent quoted lines that join with
    /// nothing at all; a newline or a space inserted here would put one in the
    /// middle of a sentence a person reads.
    #[test]
    fn continuation_lines_join_with_nothing_between_them() {
        let catalog = catalog(
            "msgid \"\"\n\"This container arrived from elsewhere, \"\n\"and the payload will carry that.\"\nmsgstr \"\"\n\"Dieser Behälter kam von anderswo, \"\n\"und die Nutzlast trägt das weiter.\"\n",
        );
        assert_eq!(
            single(
                &catalog,
                "This container arrived from elsewhere, and the payload will carry that."
            ),
            Some("Dieser Behälter kam von anderswo, und die Nutzlast trägt das weiter.")
        );
    }

    /// Would catch escapes reaching the window as backslash-n rather than as
    /// the line break the translator wrote.
    #[test]
    fn escapes_are_read() {
        assert_eq!(unquote(r#""a\nb\t\"c\\d""#), "a\nb\t\"c\\d");
    }

    /// Would catch an unknown escape quietly losing its character.
    ///
    /// Leaving the backslash in place makes the mistake visible in the window,
    /// where somebody will report it, rather than deleting a letter.
    #[test]
    fn an_unknown_escape_keeps_its_backslash() {
        assert_eq!(unquote(r#""a\qb""#), r"a\qb");
    }

    /// Would catch a context being ignored, which would make two entries that
    /// exist precisely because one English word means two things collapse into
    /// whichever was parsed last.
    #[test]
    fn a_context_keeps_two_senses_apart() {
        let catalog = catalog(
            "msgctxt \"verb\"\nmsgid \"Open\"\nmsgstr \"Öffnen\"\n\nmsgctxt \"adjective\"\nmsgid \"Open\"\nmsgstr \"Offen\"\n",
        );
        assert_eq!(catalog.keyed.len(), 2, "both senses are kept");
        assert!(catalog.plain.is_empty(), "and neither is filed without one");
    }

    /// Would catch a plural entry losing a form, or the forms being filed out
    /// of the order the rule indexes them by.
    #[test]
    fn plural_forms_are_kept_in_rule_order() {
        let catalog = catalog(
            "msgid \"{n} byte\"\nmsgid_plural \"{n} bytes\"\nmsgstr[0] \"{n} Byte\"\nmsgstr[1] \"{n} Bytes\"\n",
        );
        match catalog.plain.get("{n} byte") {
            Some(Message::Plural(forms)) => assert_eq!(forms, &["{n} Byte", "{n} Bytes"]),
            _ => panic!("the plural entry was not filed as one"),
        }
    }

    /// Would catch the plural rule check accepting a language whose plurals
    /// this module cannot choose between, which would show a German singular
    /// for every count above one.
    #[test]
    fn only_the_two_form_rule_is_accepted() {
        assert!(two_form_rule("Plural-Forms: nplurals=2; plural=(n != 1);\n"));
        assert!(two_form_rule("Plural-Forms: nplurals=2; plural=n != 1;\n"));
        assert!(
            !two_form_rule("Plural-Forms: nplurals=3; plural=(n%10==1 && n%100!=11) ? 0 : 1;\n"),
            "three forms cannot be chosen between by a rule this module does not evaluate"
        );
    }

    /// Would catch a substituted value being scanned for placeholders, which
    /// is the defect a chain of `replace` calls would have.
    ///
    /// Every message that names a file names one whose name came from whoever
    /// made the container. A payload called `{reason}.pdf` would otherwise
    /// rewrite the sentence it appears in.
    #[test]
    fn a_substituted_value_is_not_substituted_into() {
        assert_eq!(
            fill(
                "{file} was extracted, and the system would not open it: {reason}",
                &[("file", "{reason}.pdf"), ("reason", "no application")],
            ),
            "{reason}.pdf was extracted, and the system would not open it: no application"
        );
    }

    /// Would catch a placeholder a translator invented being dropped silently,
    /// leaving a sentence with a hole in it and nothing to explain the hole.
    #[test]
    fn an_unknown_placeholder_keeps_its_braces() {
        assert_eq!(fill("a {nope} b", &[("x", "y")]), "a {nope} b");
        assert_eq!(fill("a {unclosed b", &[]), "a {unclosed b");
    }

    /// Would catch a locale name in any of the three platforms' spellings
    /// failing to match a catalogue tag.
    #[test]
    fn locale_names_normalise_to_one_spelling() {
        assert_eq!(normalise("de_DE.UTF-8").as_deref(), Some("de-de"));
        assert_eq!(normalise("de-DE").as_deref(), Some("de-de"));
        assert_eq!(normalise("de").as_deref(), Some("de"));
        assert_eq!(normalise("de_AT@euro").as_deref(), Some("de-at"));
    }

    /// Would catch `C` being treated as a language, which would send a lookup
    /// after a catalogue no fleet application will ever ship and, worse, would
    /// let a `LANG` beneath an `LC_ALL=C` decide the language after POSIX says
    /// the question is settled.
    #[test]
    fn the_c_locale_is_not_a_language() {
        assert_eq!(normalise("C"), None);
        assert_eq!(normalise("POSIX"), None);
        assert_eq!(normalise(""), None);
    }

    /// Would catch a regional locale falling all the way through to English
    /// when the language it belongs to is on the shelf.
    ///
    /// Austria and Switzerland read the German catalogue; this is the whole
    /// difference between shipping German and shipping German to Germany.
    #[test]
    fn a_region_falls_back_to_its_language() {
        let available = [("de", "msgid \"Save\"\nmsgstr \"Sichern\"\n")];
        assert_eq!(choose(&available, "de-AT").map(|(tag, _)| tag), Some("de"));
        assert_eq!(choose(&available, "de").map(|(tag, _)| tag), Some("de"));
        assert_eq!(choose(&available, "fr-FR").map(|(tag, _)| tag), None);
    }

    /// Would catch an exact regional catalogue being passed over for the bare
    /// language, which is the case an `en-GB` spelling catalogue would exist
    /// for and the reason `choose` tries the full tag first.
    #[test]
    fn an_exact_region_wins_over_its_language() {
        let available = [("en", "msgid \"Colour\"\nmsgstr \"Color\"\n"), ("en-GB", "")];
        assert_eq!(
            choose(&available, "en_GB.UTF-8").map(|(tag, _)| tag),
            Some("en-GB")
        );
    }
}
