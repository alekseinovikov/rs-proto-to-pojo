use std::path::{Path, PathBuf};

use parser::{FieldType, ProtoModel, ScalarType, TypeDecl, parse_proto_file};

#[derive(Debug)]
pub enum GenerateError {
    Parse(parser::ParseError),
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerateError::Parse(e) => write!(f, "parse error: {}", e),
        }
    }
}

impl std::error::Error for GenerateError {}

impl From<parser::ParseError> for GenerateError {
    fn from(e: parser::ParseError) -> Self {
        GenerateError::Parse(e)
    }
}

/// Generate Java sources from a .proto file path.
/// Returns a list of tuples: (relative_file_path, file_content).
/// The relative_file_path uses '/' separators and includes package directories if present.
pub fn generate_java_from_proto<P: AsRef<Path>>(
    proto_path: P,
) -> Result<Vec<(String, String)>, GenerateError> {
    let model = parse_proto_file(proto_path)?;
    Ok(generate_java_from_model(&model))
}

/// Internal: generate Java source files from the ProtoModel
pub fn generate_java_from_model(model: &ProtoModel) -> Vec<(String, String)> {
    let pkg_path = model.package.as_ref().map(|p| p.replace('.', "/"));

    let mut out = Vec::new();
    for t in &model.types {
        match t {
            TypeDecl::Message(m) => {
                let code = render_message_class(model.package.as_deref(), m);
                let file_name = format!("{}.java", m.name);
                let rel = if let Some(ref pp) = pkg_path {
                    PathBuf::from(pp).join(&file_name)
                } else {
                    PathBuf::from(&file_name)
                };
                out.push((rel_to_string(&rel), code));
            }
            TypeDecl::Enum(e) => {
                let code = render_enum(model.package.as_deref(), e);
                let file_name = format!("{}.java", e.name);
                let rel = if let Some(ref pp) = pkg_path {
                    PathBuf::from(pp).join(&file_name)
                } else {
                    PathBuf::from(&file_name)
                };
                out.push((rel_to_string(&rel), code));
            }
        }
    }
    out
}

fn rel_to_string(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    s.replace('\\', "/")
}

fn render_package_line(pkg: Option<&str>) -> String {
    match pkg {
        Some(p) if !p.is_empty() => format!("package {};\n\n", p),
        _ => String::new(),
    }
}

fn render_enum(pkg: Option<&str>, e: &parser::Enum) -> String {
    let mut s = String::new();
    s.push_str(&render_package_line(pkg));
    s.push_str(&format!("public enum {} {{\n", e.name));
    for (idx, v) in e.values.iter().enumerate() {
        let sep = if idx + 1 == e.values.len() { ";" } else { "," };
        s.push_str(&format!("    {}({}){}\n", v.name, v.number, sep));
    }
    s.push_str("\n    private final int number;\n");
    s.push_str(&format!(
        "    {}(int number) {{ this.number = number; }}\n",
        e.name
    ));
    s.push_str("    public int getNumber() { return number; }\n");
    s.push_str("}\n");
    s
}

fn render_message_class(pkg: Option<&str>, m: &parser::Message) -> String {
    let mut s = String::new();
    s.push_str(&render_package_line(pkg));
    s.push_str(&format!("public class {} {{\n", m.name));

    // fields
    for f in &m.fields {
        let jt = java_type_for(&f.ty);
        s.push_str(&format!("    private {} {};\n", jt, f.name));
    }
    s.push('\n');

    // no-arg constructor
    s.push_str(&format!("    public {}() {{}}\n\n", m.name));

    // getters/setters
    for f in &m.fields {
        let jt = java_type_for(&f.ty);
        let cap = capitalize(&f.name);
        s.push_str(&format!(
            "    public {} get{}() {{ return this.{}; }}\n",
            jt, cap, f.name
        ));
        s.push_str(&format!(
            "    public void set{}({} value) {{ this.{} = value; }}\n\n",
            cap, jt, f.name
        ));
    }

    s.push_str("}\n");
    s
}

fn capitalize(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + it.as_str(),
    }
}

fn java_type_for(ft: &FieldType) -> String {
    match ft {
        FieldType::Scalar(st) => match st {
            ScalarType::Double => "double".into(),
            ScalarType::Float => "float".into(),
            ScalarType::Int32 | ScalarType::Sint32 | ScalarType::Sfixed32 => "int".into(),
            ScalarType::Uint32 | ScalarType::Fixed32 => "int".into(),
            ScalarType::Int64 | ScalarType::Sint64 | ScalarType::Sfixed64 => "long".into(),
            ScalarType::Uint64 | ScalarType::Fixed64 => "long".into(),
            ScalarType::Bool => "boolean".into(),
            ScalarType::String => "String".into(),
            ScalarType::Bytes => "byte[]".into(),
        },
        FieldType::Custom(name) => {
            // Use simple name portion for Java type reference in same package
            if let Some(idx) = name.rfind('.') {
                name[idx + 1..].to_string()
            } else {
                name.clone()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_from_order_proto_smoke() {
        // Use the shared fixture from the parser crate
        let path = Path::new("../parser/tests/resources/order.proto");
        let files = generate_java_from_proto(path).expect("generation should succeed");
        assert!(!files.is_empty());
        // Expect at least one class or enum generated and containing package line
        let any_has_package = files.iter().any(|(_, src)| src.contains("package "));
        assert!(any_has_package);
    }
}
