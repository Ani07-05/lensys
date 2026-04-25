pub mod claude;
pub mod code_context;
pub mod memory;
pub mod screenshot;
pub mod vision;
pub mod web_search;
pub mod wiki;

pub use claude::ClaudeClient;
pub use code_context::{get_active_code_context, get_clipboard_code_context, CodeContext};
pub use memory::QdrantClient;
pub use screenshot::{capture_screen_at_cursor, screens_differ, ScreenHash};
pub use vision::analyze_screenshot;
pub use web_search::{search_web, SearchResult};
pub use wiki::WikiManager;
