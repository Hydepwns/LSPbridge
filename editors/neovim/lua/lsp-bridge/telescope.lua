-- Telescope integration for LSP Bridge
-- Provides diagnostic picking and filtering

local M = {}

local pickers = require("telescope.pickers")
local finders = require("telescope.finders")
local conf = require("telescope.config").values
local actions = require("telescope.actions")
local action_state = require("telescope.actions.state")
local entry_display = require("telescope.pickers.entry_display")

-- Create diagnostic picker
function M.diagnostics(opts)
  opts = opts or {}
  
  -- Get all diagnostics
  local diagnostics = {}
  for bufnr, buffer_diagnostics in pairs(vim.diagnostic.get()) do
    local filename = vim.api.nvim_buf_get_name(bufnr)
    if filename ~= "" then
      for _, diag in ipairs(buffer_diagnostics) do
        table.insert(diagnostics, {
          bufnr = bufnr,
          filename = filename,
          lnum = diag.lnum,
          col = diag.col,
          severity = diag.severity,
          message = diag.message,
          source = diag.source,
          code = diag.code,
        })
      end
    end
  end
  
  -- Sort by severity and filename
  table.sort(diagnostics, function(a, b)
    if a.severity ~= b.severity then
      return a.severity < b.severity
    end
    if a.filename ~= b.filename then
      return a.filename < b.filename
    end
    return a.lnum < b.lnum
  end)
  
  local displayer = entry_display.create({
    separator = " ",
    items = {
      { width = 4 },
      { width = 30 },
      { width = 5 },
      { remaining = true },
    },
  })
  
  local make_display = function(entry)
    local severity_signs = {
      [vim.diagnostic.severity.ERROR] = "🔴",
      [vim.diagnostic.severity.WARN] = "🟡",
      [vim.diagnostic.severity.INFO] = "🔵",
      [vim.diagnostic.severity.HINT] = "💡",
    }
    
    return displayer({
      severity_signs[entry.severity] or " ",
      vim.fn.fnamemodify(entry.filename, ":t"),
      tostring(entry.lnum + 1),
      entry.message,
    })
  end
  
  pickers.new(opts, {
    prompt_title = "LSP Bridge - Diagnostics",
    finder = finders.new_table({
      results = diagnostics,
      entry_maker = function(entry)
        return {
          value = entry,
          display = make_display,
          ordinal = entry.filename .. " " .. entry.message,
          filename = entry.filename,
          lnum = entry.lnum + 1,
          col = entry.col + 1,
        }
      end,
    }),
    sorter = conf.generic_sorter(opts),
    previewer = conf.qflist_previewer(opts),
    attach_mappings = function(prompt_bufnr, map)
      actions.select_default:replace(function()
        local selection = action_state.get_selected_entry()
        actions.close(prompt_bufnr)
        
        if selection then
          -- Jump to diagnostic location
          vim.api.nvim_win_set_buf(0, selection.value.bufnr)
          vim.api.nvim_win_set_cursor(0, { selection.value.lnum + 1, selection.value.col })
        end
      end)
      
      -- Export selected diagnostics
      map("i", "<C-e>", function()
        local picker = action_state.get_current_picker(prompt_bufnr)
        local multi_selection = picker:get_multi_selection()
        
        if #multi_selection == 0 then
          multi_selection = { action_state.get_selected_entry() }
        end
        
        -- Export selected diagnostics
        local selected_diagnostics = {}
        for _, entry in ipairs(multi_selection) do
          table.insert(selected_diagnostics, entry.value)
        end
        
        M.export_selected(selected_diagnostics)
        actions.close(prompt_bufnr)
      end)
      
      return true
    end,
  }):find()
end

-- Export selected diagnostics
function M.export_selected(diagnostics)
  local lsp_bridge = require("lsp-bridge")
  
  -- Convert to LSP Bridge format
  local export_data = {
    source = "neovim-telescope",
    timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ"),
    diagnostics = {}
  }
  
  for _, diag in ipairs(diagnostics) do
    table.insert(export_data.diagnostics, {
      file = diag.filename,
      message = diag.message,
      severity = lsp_bridge.convert_severity(diag.severity),
      range = {
        start = {
          line = diag.lnum + 1,
          character = diag.col
        },
        ["end"] = {
          line = diag.lnum + 1,
          character = diag.col
        }
      },
      code = diag.code,
      source = diag.source
    })
  end
  
  -- Export to clipboard
  local cmd = {
    lsp_bridge.config.executable,
    "export",
    "--format", lsp_bridge.config.format,
    "--privacy", lsp_bridge.config.privacy_level
  }
  
  local result = vim.fn.system(cmd, vim.fn.json_encode(export_data))
  
  if vim.v.shell_error == 0 then
    vim.fn.setreg("+", result)
    vim.notify(string.format("Exported %d diagnostics to clipboard", #diagnostics), vim.log.levels.INFO)
  else
    vim.notify("Failed to export diagnostics: " .. result, vim.log.levels.ERROR)
  end
end

-- Register Telescope extension
function M.register()
  require("telescope").register_extension({
    exports = {
      lsp_bridge = M.diagnostics,
    },
  })
end

return M