use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::{env, fs};

use serde_json::{Map, Value};

const LOCALES: &str = "locales";
const SOURCE: &str = "en";
const KEYWORDS: [&str; 48] = [
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "static", "struct", "trait", "true", "try", "type", "typeof", "unsafe", "unsized", "use",
    "virtual", "where", "while", "yield",
];
const UNUSABLE: [&str; 3] = ["self", "super", "crate"];

struct Locale {
    code: String,
    table: Map<String, Value>,
}

fn main() {
    println!("cargo:rerun-if-changed={LOCALES}");

    let locales = read_locales(Path::new(LOCALES));
    let source = locales
        .iter()
        .find(|locale| locale.code == SOURCE)
        .unwrap_or_else(|| panic!("{LOCALES}/{SOURCE}.json is required"));

    let mut errors = Vec::new();
    for locale in &locales {
        if locale.code != SOURCE {
            check(&source.table, &locale.table, &locale.code, "", &mut errors);
        }
    }
    if !errors.is_empty() {
        panic!("invalid translations:\n  {}", errors.join("\n  "));
    }

    let mut out = String::new();
    write_structs(&mut out, "Strings", &source.table);
    write_language(&mut out, &locales);
    for locale in &locales {
        let _ = write!(out, "static {}: Strings = ", static_name(&locale.code));
        write_value(
            &mut out,
            "Strings",
            &source.table,
            &locale.table,
            &locale.code,
            "",
        );
        out.push_str(";\n");
    }

    let path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("strings.rs");
    fs::write(path, out).unwrap();

    embed_windows_resources();
}

#[cfg(windows)]
fn embed_windows_resources() {
    const ICON: &str = "../../assets/logo/solideoworship.ico";

    println!("cargo:rerun-if-changed={ICON}");
    winresource::WindowsResource::new()
        .set_icon(ICON)
        .set("ProductName", "SolideoWorship")
        .set("FileDescription", "SolideoWorship")
        .compile()
        .expect("embed the Windows icon and version info");
}

#[cfg(not(windows))]
fn embed_windows_resources() {}

fn read_locales(dir: &Path) -> Vec<Locale> {
    let mut locales: Vec<Locale> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "json").then_some(path)
        })
        .map(|path| {
            let code = path.file_stem().unwrap().to_string_lossy().into_owned();
            assert!(
                !code.is_empty() && code.bytes().all(|byte| byte.is_ascii_lowercase()),
                "{}: locale file names must be lowercase language codes",
                path.display()
            );
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            let Value::Object(table) = serde_json::from_str(&text)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            else {
                panic!("{}: the top level must be an object", path.display());
            };
            Locale { code, table }
        })
        .collect();

    locales.sort_by(|a, b| (a.code != SOURCE, &a.code).cmp(&(b.code != SOURCE, &b.code)));
    locales
}

fn check(
    source: &Map<String, Value>,
    target: &Map<String, Value>,
    code: &str,
    prefix: &str,
    errors: &mut Vec<String>,
) {
    for (key, value) in target {
        let path = format!("{prefix}{key}");
        match (source.get(key), value) {
            (None, _) => errors.push(format!("{code}: unknown key `{path}`")),
            (Some(Value::Object(inner)), Value::Object(value)) => {
                check(inner, value, code, &format!("{path}."), errors);
            }
            (Some(Value::String(english)), Value::String(text)) => {
                if text.trim() != text {
                    errors.push(format!(
                        "{code}: `{path}` must not start or end with whitespace"
                    ));
                }
                if !text.is_empty() && placeholders(english) != placeholders(text) {
                    errors.push(format!(
                        "{code}: `{path}` must use the placeholders {:?}",
                        placeholders(english)
                    ));
                }
            }
            _ => errors.push(format!(
                "{code}: `{path}` has a different shape than in {SOURCE}"
            )),
        }
    }
}

fn placeholders(text: &str) -> BTreeSet<&str> {
    text.split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(name, _)| name))
        .collect()
}

fn write_structs(out: &mut String, name: &str, table: &Map<String, Value>) {
    let _ = writeln!(out, "#[allow(non_camel_case_types)]\npub struct {name} {{");
    for (key, value) in table {
        let field = field_name(key);
        match value {
            Value::String(text) => {
                assert!(
                    text.trim() == text,
                    "{SOURCE}: `{key}` must not start or end with whitespace"
                );
                let _ = writeln!(out, "    pub {field}: &'static str,");
            }
            Value::Object(_) => {
                let _ = writeln!(out, "    pub {field}: {},", nested_name(name, key));
            }
            _ => panic!("{SOURCE}: `{key}` must be a string or an object"),
        }
    }
    out.push_str("}\n");

    for (key, value) in table {
        if let Value::Object(inner) = value {
            write_structs(out, &nested_name(name, key), inner);
        }
    }
}

fn write_value(
    out: &mut String,
    name: &str,
    source: &Map<String, Value>,
    target: &Map<String, Value>,
    code: &str,
    prefix: &str,
) {
    let _ = write!(out, "{name} {{");
    for (key, value) in source {
        let path = format!("{prefix}{key}");
        let _ = write!(out, " {}: ", field_name(key));
        match value {
            Value::String(english) => {
                let text = match target.get(key) {
                    Some(Value::String(text)) if !text.is_empty() => text,
                    _ => {
                        println!(
                            "cargo:warning={code}: `{path}` is not translated, using {SOURCE}"
                        );
                        english
                    }
                };
                let _ = write!(out, "{text:?},");
            }
            Value::Object(inner) => {
                let empty = Map::new();
                let target = match target.get(key) {
                    Some(Value::Object(target)) => target,
                    _ => &empty,
                };
                write_value(
                    out,
                    &nested_name(name, key),
                    inner,
                    target,
                    code,
                    &format!("{path}."),
                );
                out.push(',');
            }
            _ => unreachable!(),
        }
    }
    out.push_str(" }");
}

fn write_language(out: &mut String, locales: &[Locale]) {
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\npub enum Language {\n");
    for locale in locales {
        let _ = writeln!(out, "    {},", variant_name(&locale.code));
    }
    out.push_str("}\n\nimpl Language {\n");

    let _ = writeln!(out, "    pub const ALL: [Self; {}] = [", locales.len());
    for locale in locales {
        let _ = writeln!(out, "        Self::{},", variant_name(&locale.code));
    }
    out.push_str("    ];\n\n    pub fn code(self) -> &'static str {\n        match self {\n");
    for locale in locales {
        let _ = writeln!(
            out,
            "            Self::{} => {:?},",
            variant_name(&locale.code),
            locale.code
        );
    }
    out.push_str("        }\n    }\n\n    pub fn strings(self) -> &'static Strings {\n        match self {\n");
    for locale in locales {
        let _ = writeln!(
            out,
            "            Self::{} => &{},",
            variant_name(&locale.code),
            static_name(&locale.code)
        );
    }
    out.push_str("        }\n    }\n}\n");
}

fn field_name(key: &str) -> String {
    assert!(
        key.bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
            && key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && !key.contains("__")
            && !key.ends_with('_'),
        "{SOURCE}: key `{key}` must be snake_case without double or trailing underscores"
    );
    assert!(
        !UNUSABLE.contains(&key),
        "{SOURCE}: `{key}` cannot be used as a key"
    );

    if KEYWORDS.contains(&key) {
        format!("r#{key}")
    } else {
        key.to_owned()
    }
}

fn nested_name(parent: &str, key: &str) -> String {
    format!("{parent}__{key}")
}

fn variant_name(code: &str) -> String {
    camel_case(code)
}

fn static_name(code: &str) -> String {
    code.to_ascii_uppercase()
}

fn camel_case(text: &str) -> String {
    text.split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect()
}
