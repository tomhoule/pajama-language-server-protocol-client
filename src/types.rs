#[derive(Debug)]
pub struct Position {
    pub line: u64,
    pub character: u64,
}

/**
 * The end position is exclusive.
 */
pub struct Range {
    start: Position,
    end: Position,
}

pub struct Location {
    uri: String,
    range: Range,
}

pub struct Diagnostic {
    range: Range,
    severity: Option<DiagnosticSeverity>,
    code: Option<i32>,
    source: Option<String>,
    message: String,
}

pub enum DiagnosticSeverity {
    Error, // 1
    Warning, // 2
    Information, // 3
    Hint, // 4
}

pub struct TextEdit {
    range: Range,
    newText: String,
}

pub type URI = String;

pub struct TextDocumentIdentifier {
    uri: URI,
}

pub struct TextDocumentItem {
    uri: URI,
    languageId: String,
    version: i32,
    text: String,
}

pub struct VersionedTextDocumentIdentifier {
    uri: URI,
    languageId: String,
    text: String,
    version: i32,
}

pub struct TextDocumentPositionParams {
    textDocument: TextDocumentIdentifier,
    position: Position,
}
