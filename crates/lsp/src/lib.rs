//! Edge Language LSP Server
#![warn(
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    rustdoc::all
)]
#![deny(unused_must_use, rust_2018_idioms)]

mod diagnostics;

use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::{
    jsonrpc::Result,
    lsp_types::{
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
        InitializedParams, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
        Url,
    },
    Client, LanguageServer, LspService, Server,
};

/// State shared across the LSP server.
#[derive(Debug)]
struct State {
    // Reserved for future use (e.g., caching ASTs, symbol tables).
}

/// The Edge language server.
#[derive(Debug)]
pub struct EdgeLanguageServer {
    client: Client,
    #[allow(dead_code)]
    state: Arc<RwLock<State>>,
}

impl EdgeLanguageServer {
    /// Create a new language server instance.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(State {})),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, source: &str) {
        let diags = diagnostics::check(source);
        self.client.publish_diagnostics(uri, diags, None).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for EdgeLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        tracing::info!("Edge LSP server initialized");
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.publish_diagnostics(params.text_document.uri, &params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // We use full sync, so there's always exactly one change with the full text.
        if let Some(change) = params.content_changes.into_iter().last() {
            self.publish_diagnostics(params.text_document.uri, &change.text)
                .await;
        }
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Start the LSP server on stdin/stdout.
pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(EdgeLanguageServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
