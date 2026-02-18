//! Command-line interface for JustHTML.
//!
//! Equivalent to Python's `__main__.py`.
//!
//! Currently provides a diagnostic tokenizer mode (--tokenize)
//! for verifying tokenizer behavior. Full parsing with selectors
//! and output formatting will be added once the tree builder is ported.

use std::fs;
use std::io::{self, Read};
use std::process;

use clap::{Parser, ValueEnum};

use justhtml::tokenizer::{TokenSink, Tokenizer, TokenizerOpts};
use justhtml::tokens::{Token, TokenSinkResult};

/// Parse HTML5 and output text, pretty-printed HTML, or Markdown.
#[derive(Parser, Debug)]
#[command(
    name = "justhtml",
    version,
    about = "Parse HTML5 and output text, pretty-printed HTML, or Markdown.",
    after_help = "Examples:\n  \
        justhtml page.html\n  \
        curl -s https://example.com | justhtml -\n  \
        justhtml page.html --selector 'main p' --format text\n  \
        justhtml page.html --selector 'a' --format html\n  \
        justhtml page.html --selector 'article' --format markdown\n  \
        justhtml page.html --tokenize  (diagnostic: dump tokens)"
)]
struct Cli {
    /// HTML file to parse, or '-' to read from stdin
    path: Option<String>,

    /// CSS selector for choosing nodes (defaults to the document root)
    #[arg(long)]
    selector: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Html)]
    format: OutputFormat,

    /// Only output the first matching node
    #[arg(long)]
    first: bool,

    /// Parse input as an HTML fragment (context: <div>)
    #[arg(long)]
    fragment: bool,

    /// Text-only: join string between text nodes
    #[arg(long, default_value = " ")]
    separator: String,

    /// Text-only: strip each text node and drop empty segments (default)
    #[arg(long, default_value_t = true)]
    strip: bool,

    /// Text-only: preserve text node whitespace
    #[arg(long, conflicts_with = "strip")]
    no_strip: bool,

    /// Diagnostic mode: tokenize only and print each token
    #[arg(long)]
    tokenize: bool,

    /// Collect parse errors and print them to stderr
    #[arg(long)]
    errors: bool,
}

/// Output format choices.
#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Html,
    Text,
    Markdown,
}

/// A simple token collector for diagnostic output.
struct DiagnosticSink {
    tokens: Vec<Token>,
}

impl DiagnosticSink {
    fn new() -> Self {
        Self { tokens: Vec::new() }
    }
}

impl TokenSink for DiagnosticSink {
    fn process_token(&mut self, token: Token) -> TokenSinkResult {
        self.tokens.push(token);
        TokenSinkResult::Continue
    }

    fn process_characters(&mut self, data: &str) -> TokenSinkResult {
        self.tokens.push(Token::Characters(
            justhtml::tokens::CharacterTokens::new(data.to_string()),
        ));
        TokenSinkResult::Continue
    }
}

/// Read HTML input from a file path or stdin.
fn read_html(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(path)
    }
}

/// Run the tokenizer in diagnostic mode, printing each token.
fn run_tokenize(html: &str, collect_errors: bool) {
    use justhtml::tokens::TagKind;

    let mut sink = DiagnosticSink::new();
    let opts = TokenizerOpts::default();
    let mut tokenizer = Tokenizer::new(Some(opts), collect_errors);
    tokenizer.run(html, &mut sink);

    for token in &sink.tokens {
        match token {
            Token::Tag(tag) => {
                let kind_str = if tag.kind == TagKind::Start {
                    "StartTag"
                } else {
                    "EndTag"
                };
                if tag.kind == TagKind::Start {
                    let mut s = format!("<{}", tag.name);
                    let mut attrs: Vec<(&String, &Option<String>)> = tag.attrs.iter().collect();
                    attrs.sort_by_key(|(k, _)| k.as_str());
                    for (name, value) in &attrs {
                        match value {
                            Some(v) => s.push_str(&format!(" {}=\"{}\"", name, v)),
                            None => s.push_str(&format!(" {}", name)),
                        }
                    }
                    if tag.self_closing {
                        s.push_str(" /");
                    }
                    s.push('>');
                    println!("{}: {}", kind_str, s);
                } else {
                    println!("{}: </{}>", kind_str, tag.name);
                }
            }
            Token::Characters(chars) => {
                let escaped = chars
                    .data
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t");
                println!("Characters: \"{}\"", escaped);
            }
            Token::Comment(comment) => {
                println!("Comment: <!--{}-->", comment.data);
            }
            Token::Doctype(dt) => {
                let name = dt.doctype.name.as_deref().unwrap_or("");
                let public = dt
                    .doctype
                    .public_id
                    .as_ref()
                    .map(|p| format!(" PUBLIC \"{}\"", p))
                    .unwrap_or_default();
                let system = dt
                    .doctype
                    .system_id
                    .as_ref()
                    .map(|s| format!(" SYSTEM \"{}\"", s))
                    .unwrap_or_default();
                println!("Doctype: <!DOCTYPE {}{}{}>", name, public, system);
            }
            Token::EOF(_) => {
                println!("EOF");
            }
        }
    }

    if collect_errors && !tokenizer.errors.is_empty() {
        eprintln!("\n--- Parse Errors ---");
        for error in &tokenizer.errors {
            eprintln!("  {}", error);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    // Determine effective strip mode
    let _strip = if cli.no_strip { false } else { cli.strip };

    // Require a path argument unless in special modes
    let path = match &cli.path {
        Some(p) => p.clone(),
        None => {
            eprintln!("Error: HTML file path is required (use '-' for stdin)");
            eprintln!("Run `justhtml --help` for usage information.");
            process::exit(1);
        }
    };

    // Read HTML input
    let html = match read_html(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    };

    // Tokenize-only diagnostic mode
    if cli.tokenize {
        run_tokenize(&html, cli.errors);
        return;
    }

    // Full parsing mode (tree builder + selector + serializer not yet implemented)
    eprintln!(
        "Note: Full parsing mode is not yet implemented. \
         Use --tokenize for diagnostic tokenizer output."
    );
    eprintln!(
        "The tree builder, selector engine, and serializer are \
         planned for the next milestone."
    );
    process::exit(2);
}
