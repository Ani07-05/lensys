pub mod claude;
pub mod code_context;
pub mod file_edit;
pub mod memory;
pub mod screenshot;
pub mod vision;
pub mod web_search;
pub mod wiki;

pub use claude::{ClaudeClient, CodeActionProposal};
pub use code_context::{get_active_code_context, get_clipboard_code_context, CodeContext};
pub use file_edit::{apply_code_action, ApplyCodeActionRequest, ApplyCodeActionResult};
pub use memory::QdrantClient;
pub use screenshot::{capture_screen_at_cursor, screens_differ, ScreenHash};
pub use vision::analyze_screenshot;
pub use web_search::{search_web, SearchResult};
pub use wiki::WikiManager;
