-- LSP Bridge for Neovim
-- Export IDE diagnostics to AI assistants

local M = {}

-- Default configuration
M.config = {
  executable = "lsp-bridge",
  format = "claude",
  privacy_level = "default",
  include_context = true,
  context_lines = 3,
  auto_export_on_save = false,
  quick_fix_threshold = 0.9,
  keymaps = {
    export = "<leader>de",
    export_clipboard = "<leader>dc",
    show_history = "<leader>dh",
    apply_fixes = "<leader>df",
    watch_start = "<leader>dw",
    watch_stop = "<leader>ds",
  },
}

-- State
local watch_job = nil
local diagnostics_cache = {}

-- Setup function
function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})
  
  -- Create commands
  vim.api.nvim_create_user_command("LspBridgeExport", M.export_diagnostics, {
    nargs = "?",
    complete = function() return {"json", "markdown", "claude"} end,
    desc = "Export diagnostics to file"
  })
  
  vim.api.nvim_create_user_command("LspBridgeClipboard", M.export_to_clipboard, {
    desc = "Export diagnostics to clipboard"
  })
  
  vim.api.nvim_create_user_command("LspBridgeHistory", M.show_history, {
    desc = "Show diagnostic history"
  })
  
  vim.api.nvim_create_user_command("LspBridgeQuickFix", M.apply_quick_fixes, {
    desc = "Apply quick fixes"
  })
  
  vim.api.nvim_create_user_command("LspBridgeWatch", M.start_watching, {
    desc = "Start watching diagnostics"
  })
  
  vim.api.nvim_create_user_command("LspBridgeStopWatch", M.stop_watching, {
    desc = "Stop watching diagnostics"
  })
  
  -- Set up keymaps
  if M.config.keymaps then
    for action, key in pairs(M.config.keymaps) do
      if key and key ~= "" then
        local cmd = ({
          export = "<cmd>LspBridgeExport<cr>",
          export_clipboard = "<cmd>LspBridgeClipboard<cr>",
          show_history = "<cmd>LspBridgeHistory<cr>",
          apply_fixes = "<cmd>LspBridgeQuickFix<cr>",
          watch_start = "<cmd>LspBridgeWatch<cr>",
          watch_stop = "<cmd>LspBridgeStopWatch<cr>",
        })[action]
        
        if cmd then
          vim.keymap.set("n", key, cmd, { desc = "LSP Bridge: " .. action })
        end
      end
    end
  end
  
  -- Auto-export on save if enabled
  if M.config.auto_export_on_save then
    vim.api.nvim_create_autocmd("BufWritePost", {
      group = vim.api.nvim_create_augroup("LspBridgeAutoExport", { clear = true }),
      callback = function()
        M.export_for_buffer(vim.api.nvim_get_current_buf())
      end,
    })
  end
  
  -- Update diagnostics cache
  vim.api.nvim_create_autocmd("DiagnosticChanged", {
    group = vim.api.nvim_create_augroup("LspBridgeDiagnostics", { clear = true }),
    callback = function()
      M.update_diagnostics_cache()
    end,
  })
  
  -- Create sign column signs
  vim.fn.sign_define("LspBridgeError", { text = "🔴", texthl = "DiagnosticError" })
  vim.fn.sign_define("LspBridgeWarn", { text = "🟡", texthl = "DiagnosticWarn" })
  vim.fn.sign_define("LspBridgeInfo", { text = "🔵", texthl = "DiagnosticInfo" })
  vim.fn.sign_define("LspBridgeHint", { text = "💡", texthl = "DiagnosticHint" })
end

-- Convert Neovim diagnostics to LSP Bridge format
function M.convert_diagnostics()
  local diagnostics = {}
  
  for bufnr, buffer_diagnostics in pairs(vim.diagnostic.get()) do
    local filename = vim.api.nvim_buf_get_name(bufnr)
    if filename ~= "" then
      for _, diag in ipairs(buffer_diagnostics) do
        table.insert(diagnostics, {
          file = filename,
          message = diag.message,
          severity = M.convert_severity(diag.severity),
          range = {
            start = {
              line = diag.lnum + 1,
              character = diag.col
            },
            ["end"] = {
              line = diag.end_lnum and diag.end_lnum + 1 or diag.lnum + 1,
              character = diag.end_col or diag.col
            }
          },
          code = diag.code,
          source = diag.source
        })
      end
    end
  end
  
  return {
    source = "neovim",
    timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ"),
    diagnostics = diagnostics
  }
end

-- Convert severity levels
function M.convert_severity(severity)
  local severities = {
    [vim.diagnostic.severity.ERROR] = "Error",
    [vim.diagnostic.severity.WARN] = "Warning",
    [vim.diagnostic.severity.INFO] = "Information",
    [vim.diagnostic.severity.HINT] = "Hint",
  }
  return severities[severity] or "Information"
end

-- Export diagnostics
function M.export_diagnostics(opts)
  local format = opts.args ~= "" and opts.args or M.config.format
  
  -- Get output file
  local output_file = vim.fn.input("Save diagnostics to: ", "diagnostics." .. (format == "json" and "json" or "md"), "file")
  if output_file == "" then
    return
  end
  
  local diagnostics = M.convert_diagnostics()
  local cmd = {
    M.config.executable,
    "export",
    "--format", format,
    "--privacy", M.config.privacy_level,
    "--output", output_file
  }
  
  if M.config.include_context then
    table.insert(cmd, "--include-context")
    table.insert(cmd, "--context-lines")
    table.insert(cmd, tostring(M.config.context_lines))
  end
  
  -- Run command with diagnostics as input
  local result = vim.fn.system(cmd, vim.fn.json_encode(diagnostics))
  
  if vim.v.shell_error == 0 then
    vim.notify("Diagnostics exported to " .. output_file, vim.log.levels.INFO)
  else
    vim.notify("Failed to export diagnostics: " .. result, vim.log.levels.ERROR)
  end
end

-- Export to clipboard
function M.export_to_clipboard()
  local diagnostics = M.convert_diagnostics()
  local cmd = {
    M.config.executable,
    "export",
    "--format", M.config.format,
    "--privacy", M.config.privacy_level
  }
  
  if M.config.include_context then
    table.insert(cmd, "--include-context")
    table.insert(cmd, "--context-lines")
    table.insert(cmd, tostring(M.config.context_lines))
  end
  
  local result = vim.fn.system(cmd, vim.fn.json_encode(diagnostics))
  
  if vim.v.shell_error == 0 then
    vim.fn.setreg("+", result)
    vim.notify("Diagnostics copied to clipboard", vim.log.levels.INFO)
  else
    vim.notify("Failed to export diagnostics: " .. result, vim.log.levels.ERROR)
  end
end

-- Show diagnostic history
function M.show_history()
  local cmd = {
    M.config.executable,
    "history",
    "trends",
    "--format", "json"
  }
  
  local result = vim.fn.system(cmd)
  
  if vim.v.shell_error == 0 then
    local data = vim.fn.json_decode(result)
    
    -- Create a floating window to display history
    local buf = vim.api.nvim_create_buf(false, true)
    local lines = {
      "Diagnostic History",
      "==================",
      "",
      string.format("Health Score: %.1f%%", (data.health_score or 0) * 100),
      string.format("Error Velocity: %.1f errors/hour", data.error_velocity or 0),
      string.format("Warning Velocity: %.1f warnings/hour", data.warning_velocity or 0),
      string.format("Trend: %s", data.trend_direction or "Stable"),
      "",
      "Hot Spots:",
      "----------"
    }
    
    if data.hot_spots then
      for i, spot in ipairs(data.hot_spots) do
        if i <= 5 then
          table.insert(lines, string.format("%d. %s - %d errors, %d warnings",
            i, vim.fn.fnamemodify(spot.file_path, ":t"),
            spot.last_error_count or 0,
            spot.last_warning_count or 0))
        end
      end
    end
    
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
    
    -- Create floating window
    local width = 60
    local height = #lines + 2
    local win = vim.api.nvim_open_win(buf, true, {
      relative = "editor",
      width = width,
      height = height,
      col = (vim.o.columns - width) / 2,
      row = (vim.o.lines - height) / 2,
      style = "minimal",
      border = "rounded",
      title = " LSP Bridge History ",
      title_pos = "center",
    })
    
    -- Set buffer options
    vim.api.nvim_buf_set_option(buf, "modifiable", false)
    vim.api.nvim_buf_set_keymap(buf, "n", "q", "<cmd>close<cr>", { noremap = true })
    vim.api.nvim_buf_set_keymap(buf, "n", "<Esc>", "<cmd>close<cr>", { noremap = true })
  else
    vim.notify("Failed to get history: " .. result, vim.log.levels.ERROR)
  end
end

-- Apply quick fixes
function M.apply_quick_fixes()
  -- First do a dry run
  local cmd = {
    M.config.executable,
    "quick-fix",
    "apply",
    "--threshold", tostring(M.config.quick_fix_threshold),
    "--dry-run"
  }
  
  local diagnostics = M.convert_diagnostics()
  local result = vim.fn.system(cmd, vim.fn.json_encode(diagnostics))
  
  if vim.v.shell_error ~= 0 then
    vim.notify("Failed to analyze fixes: " .. result, vim.log.levels.ERROR)
    return
  end
  
  -- Count available fixes
  local fix_count = 0
  for line in result:gmatch("[^\r\n]+") do
    if line:match("Would fix:") then
      fix_count = fix_count + 1
    end
  end
  
  if fix_count == 0 then
    vim.notify("No fixes available with sufficient confidence", vim.log.levels.INFO)
    return
  end
  
  -- Ask for confirmation
  local confirm = vim.fn.confirm(
    string.format("Apply %d fixes with confidence >= %.1f?", fix_count, M.config.quick_fix_threshold),
    "&Yes\n&No", 2)
  
  if confirm == 1 then
    -- Apply fixes
    cmd[#cmd] = nil  -- Remove --dry-run
    result = vim.fn.system(cmd, vim.fn.json_encode(diagnostics))
    
    if vim.v.shell_error == 0 then
      vim.notify("Successfully applied fixes", vim.log.levels.INFO)
      -- Reload affected buffers
      vim.cmd("checktime")
    else
      vim.notify("Failed to apply fixes: " .. result, vim.log.levels.ERROR)
    end
  end
end

-- Start watching diagnostics
function M.start_watching()
  if watch_job then
    vim.notify("Already watching diagnostics", vim.log.levels.WARN)
    return
  end
  
  local output_buf = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_name(output_buf, "LSP Bridge Watch")
  
  watch_job = vim.fn.jobstart({
    M.config.executable,
    "watch",
    "--format", M.config.format,
    "--privacy", M.config.privacy_level
  }, {
    on_stdout = function(_, data)
      vim.api.nvim_buf_set_lines(output_buf, -1, -1, false, data)
    end,
    on_exit = function()
      watch_job = nil
      vim.notify("Stopped watching diagnostics", vim.log.levels.INFO)
    end,
  })
  
  -- Open buffer in split
  vim.cmd("split")
  vim.api.nvim_set_current_buf(output_buf)
  vim.cmd("resize 15")
  
  vim.notify("Started watching diagnostics", vim.log.levels.INFO)
end

-- Stop watching diagnostics
function M.stop_watching()
  if not watch_job then
    vim.notify("Not watching diagnostics", vim.log.levels.WARN)
    return
  end
  
  vim.fn.jobstop(watch_job)
  watch_job = nil
end

-- Export for specific buffer
function M.export_for_buffer(bufnr)
  local diagnostics = vim.diagnostic.get(bufnr)
  if #diagnostics == 0 then
    return
  end
  
  -- Store in cache for history
  local filename = vim.api.nvim_buf_get_name(bufnr)
  diagnostics_cache[filename] = {
    timestamp = os.time(),
    diagnostics = diagnostics
  }
end

-- Update diagnostics cache
function M.update_diagnostics_cache()
  -- Update statusline
  vim.cmd("redrawstatus")
end

-- Statusline component
function M.statusline()
  local error_count = 0
  local warning_count = 0
  
  for _, diagnostics in pairs(vim.diagnostic.get()) do
    for _, diag in ipairs(diagnostics) do
      if diag.severity == vim.diagnostic.severity.ERROR then
        error_count = error_count + 1
      elseif diag.severity == vim.diagnostic.severity.WARN then
        warning_count = warning_count + 1
      end
    end
  end
  
  if error_count > 0 then
    return string.format("🔴%d 🟡%d", error_count, warning_count)
  elseif warning_count > 0 then
    return string.format("🟡%d", warning_count)
  else
    return "✅"
  end
end

return M