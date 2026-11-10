"""隔离测量使用的固定配置和 PTY 输出样本。"""

SETTINGS = """[general]
login_item = false
restore_session = false
show_warning_before_quitting = false
should_confirm_close_session = false
show_changelog_after_update = false
[privacy]
telemetry_enabled = false
crash_reporting_enabled = false
[appearance.text]
font_name = "Hack"
font_size = 14.0
[appearance.window]
open_windows_at_custom_size = true
new_windows_num_columns = 120
new_windows_num_rows = 40
override_opacity = 100
[appearance.vertical_tabs]
enabled = false
[terminal]
maximum_grid_size = 50000
"""

RC = r"""export HISTFILE="$HOME/.zsh_history"
zmodload zsh/datetime
function tinywarp_perf_prompt_ready() { print -r -- "$$ $EPOCHREALTIME" >> "$HOME/prompt-ready" }
precmd_functions+=(tinywarp_perf_prompt_ready)
function TRAPUSR1() {
    /usr/bin/awk 'BEGIN {for(i=1;i<=100000;i++) printf "tinywarp-output-%06d abcdefghijklmnopqrstuvwxyz0123456789\n",i}'
    print done > "$HOME/output-done"
}
function TRAPUSR2() { print done > "$HOME/fixture-response" }
"""
