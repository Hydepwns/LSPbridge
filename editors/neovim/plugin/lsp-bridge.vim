" LSP Bridge plugin for Neovim
" Ensure we're in Neovim
if !has('nvim-0.5')
  echohl WarningMsg
  echom "LSP Bridge requires Neovim 0.5 or later"
  echohl None
  finish
endif

" Prevent loading twice
if exists('g:loaded_lsp_bridge')
  finish
endif
let g:loaded_lsp_bridge = 1

" Initialize the plugin
lua require('lsp-bridge').setup()