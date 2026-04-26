# Cluddy

A floating AI buddy for developers that provides intelligent assistance directly on your desktop with voice interaction, code analysis, and AI-powered problem solving.

## Overview

Cluddy is a lightweight desktop application that keeps an always-on-top AI assistant at your fingertips. Built with Tauri for cross-platform compatibility, it combines voice interaction via Vapi, AI models (Claude, Groq), and developer-centric features like code context awareness and web search to enhance your development workflow.

## Features

- **Floating Interface**: Always-on-top orb UI that never gets in the way until you need it
- **Voice Interaction**: Speak naturally to your AI assistant using Vapi voice calling
- **AI-Powered Assistance**: Integration with Claude and Groq for intelligent responses
- **Code Context Awareness**: Automatically captures and understands your active code context
- **Screenshot Analysis**: Analyze visual content with vision capabilities
- **Web Search**: Search the web in real-time for information
- **Code Editing**: Receive and apply code suggestions directly
- **Memory Management**: Persistent memory system using Qdrant vector database
- **Global Shortcuts**: Quick access from anywhere on your system
- **Clipboard Integration**: Include clipboard content in your queries

## Tech Stack

- **Frontend**: React 18, TypeScript, Vite, TailwindCSS
- **Backend**: Rust with Tauri v2 framework
- **Voice**: Vapi.ai
- **AI Models**: Claude, Groq
- **Search**: Tavily
- **Vector DB**: Qdrant
- **Build Tools**: Tauri CLI, npm/pnpm

## Prerequisites

- Node.js (v16 or later) with npm or pnpm
- Rust 1.70+ (for building Tauri app)
- Tauri CLI
- API Keys:
  - Vapi (for voice calling)
  - Vapi Assistant ID
  - Claude API key (optional, can use Groq)
  - Groq API key (optional)
  - Tavily API key (for web search, optional)
  - Qdrant URL and API key (for memory management)

## Installation

### 1. Clone the Repository

```bash
git clone <repository-url>
cd lensys
```

### 2. Install Frontend Dependencies

```bash
npm install
# or
pnpm install
```

### 3. Install Tauri CLI (if not already installed)

```bash
npm install -g @tauri-apps/cli
# or
pnpm add -g @tauri-apps/cli
```

### 4. Configure Environment Variables

Create a `.env` file in the root directory:

```env
VAPI_PUBLIC_KEY=your_vapi_public_key
VAPI_ASSISTANT_ID=your_vapi_assistant_id
CLAUDE_API_KEY=your_claude_api_key
GROQ_API_KEY=your_groq_api_key
TAVILY_API_KEY=your_tavily_api_key
QDRANT_URL=http://localhost:6333
QDRANT_API_KEY=your_qdrant_api_key
```

## Development

### Start Development Server

```bash
npm run tauri dev
```

This command:
- Starts the Vite dev server on port 1420
- Builds and runs the Tauri application in development mode
- Enables hot module reloading for the React frontend
- Watches Rust source files for changes

### Frontend-Only Development

If you want to develop just the React frontend without the full Tauri app:

```bash
npm run dev
```

The frontend will run on `http://localhost:1420`.

## Building

### Production Build

```bash
npm run build
```

This command:
- Compiles TypeScript
- Builds the React frontend with Vite
- Compiles Rust backend in release mode
- Bundles the desktop application

### Build Output

The built application will be located in `src-tauri/target/release/`.

## Project Structure

```
lensys/
├── src/                          # React frontend
│   ├── components/               # React components
│   │   ├── Orb.tsx              # Floating orb UI
│   │   └── ExpandedPanel.tsx     # Expanded view
│   ├── hooks/                    # React hooks
│   │   └── useVapi.ts           # Voice interaction hook
│   ├── App.tsx                   # Main app component
│   ├── main.tsx                  # React entry point
│   └── index.css                 # Global styles
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs              # App entry point
│   │   ├── lib.rs               # Core library
│   │   ├── api.rs               # API utilities
│   │   ├── ui.rs                # UI-related functions
│   │   └── commands/            # Tauri commands
│   │       ├── claude.rs        # Claude AI integration
│   │       ├── code_context.rs  # Code context extraction
│   │       ├── file_edit.rs     # File editing
│   │       ├── memory.rs        # Memory management
│   │       ├── screenshot.rs    # Screenshot capture
│   │       ├── vision.rs        # Vision analysis
│   │       ├── web_search.rs    # Web search
│   │       └── wiki.rs          # Wiki management
│   ├── tauri.conf.json          # Tauri configuration
│   └── Cargo.toml               # Rust dependencies
├── cmd/                          # Go utilities
│   └── lensys/
│       └── main.go
├── internal/                     # Internal Go packages
│   └── lensys/
│       ├── api.go
│       └── ui.go
├── vite.config.ts               # Vite configuration
├── tailwind.config.js           # TailwindCSS configuration
└── package.json                 # Node.js dependencies
```

## Configuration

### Tauri Configuration

Edit `src-tauri/tauri.conf.json` to customize:
- Window dimensions and behavior
- Application metadata
- Build settings

### Frontend

- **Vite Config**: `vite.config.ts` - dev server and build settings
- **TailwindCSS**: `tailwind.config.js` - styling configuration
- **TypeScript**: `tsconfig.json` and `tsconfig.node.json` - type checking

## API Endpoints and Commands

The application exposes the following Tauri commands:

- `get_env_keys` - Retrieve configured API keys status
- `get_code_context` - Extract active code context
- `get_clipboard_code_context` - Get code from clipboard
- `capture_screen_at_cursor` - Capture screenshot at cursor position
- `analyze_screenshot` - Analyze captured screenshot with vision
- `search_web` - Search the web using Tavily
- `ask_claude` - Query Claude AI
- `apply_code_action` - Apply suggested code changes
- `set_window_mode` - Change window display mode (orb, calling, expanded)
- `edit_file` - Edit project files
- `save_memory` - Store memory with Qdrant

## Usage

### Starting the Application

```bash
npm run tauri dev          # Development mode
npm run tauri build        # Production build
```

### Interaction Modes

**Orb Mode**: Floating inactive state (minimal footprint)

**Calling Mode**: Active voice conversation with transcript display

**Expanded Mode**: Full-screen interface with detailed information and controls

### Global Shortcuts

Press the configured global shortcut to activate voice interaction. The default behavior:
- From orb mode: Opens calling mode to start a voice call
- From calling/expanded mode: Returns to orb mode

## Troubleshooting

### Application Won't Start

- Ensure all required API keys are set in `.env`
- Check that Rust toolchain is properly installed: `rustc --version`
- Verify Node.js version: `node --version`

### Voice Not Working

- Confirm Vapi API keys are correct
- Check internet connection
- Verify Vapi service status

### Build Errors

Clear cache and rebuild:

```bash
rm -rf src-tauri/target
npm install
npm run build
```

### Type Errors

Run TypeScript compiler to check:

```bash
npx tsc --noEmit
```

## Performance Optimization

The production build includes:
- Code splitting and minification
- Rust release mode compilation with LTO (Link Time Optimization)
- Optimized bundle stripping
- Panic abort for smaller binary size

## Contributing

When contributing:
1. Ensure TypeScript compiles without errors
2. Test in development mode before submitting
3. Follow existing code style and patterns
4. Update this README if adding new features or commands

## License

See LICENSE file for details.

## Support

For issues, questions, or feature requests, please open an issue in the repository.
