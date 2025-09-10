Project: rs-proto-to-pojo

CLI tool written in Rust for parsing Protocol Buffers (proto3) and generating Java POJO classes from message definitions. Future code generation for other languages may be added.

⸻

Purpose
•	Provide a minimal, reliable Rust parser for .proto files (using pest).
•	Transform parsed messages into compilable Java classes.
•	Keep architecture extensible for adding more output targets later.

⸻

Repository Layout
•	Workspace
•	Root Cargo.toml defines edition 2024, resolver 3, and centralized dependencies.
•	Members: parser/.
•	Crate: parser/
•	src/lib.rs — main ProtoParser implementation and unit tests.
•	resources/proto.pest — grammar definition.
•	tests/ — integration tests + fixtures.
•	tests/resources/order.proto — shared test fixture.

⸻

Build & Test
•	Build all: cargo build
•	Build only parser: cargo build -p parser
•	Run all tests: cargo test
•	Run only parser tests: cargo test -p parser

Notes:
•	#[grammar = "resources/proto.pest"] path is relative to crate root (parser/).
•	Tests run with crate root as working directory.

⸻

Parser Rules (Key Points)
•	Entry rule: Rule::proto; expect parse to end at Rule::EOI.
•	message_body → message_element* → field, oneof, etc. Always unwrap one layer.
•	message_block allows optional trailing ;.
•	field_modifier optional: optional, required, repeated.
•	type_reference supports scalar + qualified identifiers.
•	string_value handles escapes (\xNN, \uNNNN, octal).

⸻

Code Style
•	Use rustfmt + clippy:
•	cargo fmt --all
•	cargo clippy -p parser -- -D warnings
•	Keep pest rules minimal and explicit.
•	Add new crates via [workspace.dependencies] with { workspace = true }.

⸻

Development Checklist
•	cargo build && cargo test passes at workspace root.
•	Grammar path resolves correctly.
•	Unit tests against order.proto pass.
•	New fixtures stored under tests/resources/.
•	Code compiles warning-free.

⸻

Future Work
•	Extend generator to support additional languages (besides Java).
•	Add model layer for parsed AST → codegen mapping.
•	Expand integration tests with more .proto structures.

⸻

Proto Model (IR)
•	Location: parser/src/model.rs
•	Purpose: Minimal internal representation of parsed .proto content to drive future code generation (Java POJOs, etc.).
•	Scope: We keep only package and types (messages and enums). Services, RPCs, imports, syntax, and other file-level constructs are intentionally omitted for now.

Model Overview
•	ProtoModel { package: Option<String>, types: Vec<TypeDecl> }
•	TypeDecl = Message | Enum
•	Message { name: String, fields: Vec<Field> }
•	Field { ty: FieldType, name: String, order: u32 }
•	FieldType = Scalar(ScalarType) | Custom(String)
•	ScalarType = { Double, Float, Int32, Int64, Uint32, Uint64, Sint32, Sint64, Fixed32, Fixed64, Sfixed32, Sfixed64, Bool, String, Bytes }
•	Enum { name: String, values: Vec<EnumValue> }
•	EnumValue { name: String, number: i32 }

Notes
•	Custom(String) stores fully qualified type names for user-defined message/enum types; scalars use ScalarType.
•	Fields use "order" as the numeric tag from the .proto definition.
•	The IR is intentionally small and stable to support straightforward code generation.

Verification (as of 2025-09-10)
•	cargo clippy -p parser -- -D warnings → clean
•	cargo test → all tests pass
