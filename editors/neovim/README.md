# LSP Bridge for Neovim

Export IDE diagnostics to AI assistants directly from Neovim!

## Features

- 📤 Export diagnostics in multiple formats (JSON, Markdown, Claude-optimized)
- 📋 Quick export to clipboard for AI chat interfaces
- 👀 Live diagnostic watching with auto-updating buffer
- 📊 Floating window for diagnostic history
- 🔧 Apply high-confidence fixes automatically
- 🔭 Telescope integration for advanced filtering
- 🔒 Privacy-aware filtering with three levels

## Requirements

- Neovim 0.5+
- LSP Bridge CLI (`cargo install lsp-bridge`)
- Optional: [Telescope.nvim](https://github.com/nvim-telescope/telescope.nvim) for enhanced UI

## Installation

### Using [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
  "your-org/lsp-bridge.nvim",
  config = function()
    require("lsp-bridge").setup({
      -- Configuration options
    })
  end,
}
```

### Using [packer.nvim](https://github.com/wbthomason/packer.nvim)

```lua
use {
  "your-org/lsp-bridge.nvim",
  config = function()
    require("lsp-bridge").setup()
  end
}
```

### Using vim-plug

```vim
Plug 'your-org/lsp-bridge.nvim'

" In your init.lua or after plug#end()
lua require('lsp-bridge').setup()
```

## Configuration

```lua
require("lsp-bridge").setup({
  executable = "lsp-bridge",        -- Path to lsp-bridge executable
  format = "claude",                -- Export format: "json", "markdown", "claude"
  privacy_level = "default",        -- Privacy: "default", "strict", "permissive"
  include_context = true,           -- Include code context
  context_lines = 3,                -- Number of context lines
  auto_export_on_save = false,      -- Auto-export on buffer save
  quick_fix_threshold = 0.9,        -- Minimum confidence for fixes
  keymaps = {
    export = "<leader>de",          -- Export to file
    export_clipboard = "<leader>dc", -- Export to clipboard
    show_history = "<leader>dh",    -- Show history
    apply_fixes = "<leader>df",     -- Apply quick fixes
    watch_start = "<leader>dw",     -- Start watching
    watch_stop = "<leader>ds",      -- Stop watching
  },
})
```

## Usage

### Commands

- `:LspBridgeExport [format]` - Export diagnostics to file
- `:LspBridgeClipboard` - Export to clipboard
- `:LspBridgeHistory` - Show diagnostic history in floating window
- `:LspBridgeQuickFix` - Apply quick fixes
- `:LspBridgeWatch` - Start watching diagnostics
- `:LspBridgeStopWatch` - Stop watching

### Default Keymaps

- `<leader>de` - Export diagnostics to file
- `<leader>dc` - Copy diagnostics to clipboard
- `<leader>dh` - Show diagnostic history
- `<leader>df` - Apply quick fixes
- `<leader>dw` - Start watching diagnostics
- `<leader>ds` - Stop watching

### Telescope Integration

If you have Telescope installed, you can use:

```lua
:Telescope lsp_bridge
```

This opens an interactive picker where you can:
- Filter diagnostics by severity, file, or content
- Preview diagnostic locations
- Select multiple diagnostics to export
- Press `<C-e>` to export selected items

### Statusline Integration

Add to your statusline:

```lua
-- For lualine
sections = {
  lualine_x = {
    function() return require('lsp-bridge').statusline() end
  }
}

-- For custom statusline
set statusline+=%{luaeval("require('lsp-bridge').statusline()")}
```

## Privacy Levels

- **Default**: Removes API keys, passwords, and sensitive data
- **Strict**: Additional filtering of paths and identifiers
- **Permissive**: Minimal filtering for trusted environments

## Examples

### Export current buffer errors only

```lua
vim.keymap.set("n", "<leader>ee", function()
  -- Save current format
  local lb = require("lsp-bridge")
  local old_format = lb.config.format
  
  -- Export errors only
  lb.config.format = "markdown"
  vim.cmd("LspBridgeExport")
  
  -- Restore format
  lb.config.format = old_format
end, { desc = "Export errors to markdown" })
```

### Auto-export on diagnostic change

```lua
vim.api.nvim_create_autocmd("DiagnosticChanged", {
  callback = function()
    local error_count = #vim.diagnostic.get(0, { severity = vim.diagnostic.severity.ERROR })
    if error_count > 5 then
      require("lsp-bridge").export_to_clipboard()
      vim.notify("High error count - diagnostics copied to clipboard")
    end
  end,
})
```

### Custom export format

```lua
-- Add to your LSP on_attach function
local function on_attach(client, bufnr)
  vim.keymap.set("n", "<leader>da", function()
    -- Get diagnostics for current position
    local line = vim.fn.line(".") - 1
    local diagnostics = vim.diagnostic.get(0, { lnum = line })
    
    if #diagnostics > 0 then
      -- Custom formatting
      local text = string.format(
        "Error at %s:%d - %s",
        vim.fn.expand("%:t"),
        line + 1,
        diagnostics[1].message
      )
      vim.fn.setreg("+", text)
      vim.notify("Diagnostic copied!")
    end
  end, { buffer = bufnr, desc = "Copy diagnostic at cursor" })
end
```

## Troubleshooting

1. **"lsp-bridge not found"**
   ```bash
   # Ensure lsp-bridge is installed
   cargo install lsp-bridge
   
   # Or specify full path in config
   executable = "/path/to/lsp-bridge"
   ```

2. **No diagnostics exported**
   - Ensure LSP servers are running: `:LspInfo`
   - Check for diagnostics: `:lua vim.diagnostic.get()`

3. **Telescope not working**
   - Install Telescope first
   - Register the extension:
   ```lua
   require("lsp-bridge.telescope").register()
   ```

## License

MIT