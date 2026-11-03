if status is-interactive; and isatty stdout; and test "$TERM_PROGRAM" = ghostty; and not set -q SSH_CONNECTION; and test "$NODALIX_TERMINAL_LOGO" != 0; and not set -q __NODALIX_TERMINAL_LOGO_SHOWN
    set -gx __NODALIX_TERMINAL_LOGO_SHOWN 1
    if not set -q fish_greeting
        set -g fish_greeting ""
    end
    /usr/bin/nodalix-terminal-logo --welcome
end
