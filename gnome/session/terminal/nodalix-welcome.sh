case $- in
    *i*)
        if [ "${TERM_PROGRAM:-}" = ghostty ] && [ -z "${__NODALIX_TERMINAL_LOGO_SHOWN:-}" ]; then
            export __NODALIX_TERMINAL_LOGO_SHOWN=1
            /usr/bin/nodalix-terminal-logo --welcome
        fi
        ;;
esac
