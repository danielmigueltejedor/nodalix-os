#!/usr/bin/env bash

nodalix_cpu_name() {
    grep -m1 "^model name" /proc/cpuinfo | cut -d: -f2- | xargs
}

nodalix_safe_cpu() {
    printf "%s" "$1" | tr " /()" "____" | tr -cd "[:alnum:]_.-"
}

# Legacy .patch support. Se conserva temporalmente mientras migramos apps.
nodalix_ensure_overlay_remote() {
    local remote="$1"
    local url="$2"

    if git remote get-url "$remote" >/dev/null 2>&1; then
        local current
        current="$(git remote get-url "$remote")"

        if [ "$current" != "$url" ]; then
            git remote set-url "$remote" "$url"
        fi
    else
        git remote add "$remote" "$url"
    fi
}

nodalix_fetch_overlay() {
    local remote="$1"
    local branch="$2"

    git fetch "$remote" \
        "+refs/heads/$branch:refs/remotes/$remote/$branch"
}

# Devuelve SOLO los commits del overlay cuyo cambio todavía no existe upstream.
# --cherry-pick compara equivalencia de patch, no solamente SHA.
nodalix_overlay_commits() {
    local upstream_ref="$1"
    local overlay_ref="$2"

    git rev-list \
        --reverse \
        --topo-order \
        --no-merges \
        --cherry-pick \
        --right-only \
        "$upstream_ref...$overlay_ref"
}

# Hash basado en el cambio efectivo, no en el SHA del commit.
# Un simple rebase no fuerza una build si el contenido sigue siendo idéntico.
nodalix_overlay_hash() {
    local upstream_ref="$1"
    local overlay_ref="$2"

    local commits=()
    mapfile -t commits < <(
        nodalix_overlay_commits "$upstream_ref" "$overlay_ref"
    )

    if [ "${#commits[@]}" -eq 0 ]; then
        printf "none\n"
        return
    fi

    local material=""
    local commit
    local patchid

    for commit in "${commits[@]}"; do
        patchid="$(
            git show \
                --pretty=format: \
                --binary \
                "$commit" \
            | git patch-id --stable \
            | awk "{print \$1}"
        )"

        if [ -z "$patchid" ]; then
            patchid="$commit"
        fi

        material+="${patchid}"$

    done

    printf "%s" "$material" | sha256sum | cut -d " " -f1
}

nodalix_show_overlay() {
    local upstream_ref="$1"
    local overlay_ref="$2"

    local commits=()
    mapfile -t commits < <(
        nodalix_overlay_commits "$upstream_ref" "$overlay_ref"
    )

    if [ "${#commits[@]}" -eq 0 ]; then
        echo "    Ningún cambio local pendiente."
        return
    fi

    local commit

    for commit in "${commits[@]}"; do
        printf "    %s  %s\n" \
            "$(git rev-parse --short=12 "$commit")" \
            "$(git show -s --format=%s "$commit")"
    done
}

nodalix_apply_git_overlay() {
    local upstream_ref="$1"
    local overlay_ref="$2"

    local commits=()
    mapfile -t commits < <(
        nodalix_overlay_commits "$upstream_ref" "$overlay_ref"
    )

    if [ "${#commits[@]}" -eq 0 ]; then
        echo "==> No hay cambios Nodalix pendientes de upstream."
        return 0
    fi

    echo "==> Aplicando overlay Git Nodalix..."

    local commit

    for commit in "${commits[@]}"; do
        echo "    $(git rev-parse --short=12 "$commit")  $(git show -s --format=%s "$commit")"

        if ! git cherry-pick --empty=drop "$commit"; then
            git cherry-pick --abort >/dev/null 2>&1 || true

            echo
            echo "ERROR: el overlay Nodalix ya no aplica limpiamente."
            echo "Commit conflictivo: $commit"
            echo "La instalación activa NO se ha modificado."
            return 1
        fi
    done
}

nodalix_finish_update() {
    if command -v nodalix-apps-icons >/dev/null 2>&1; then
        nodalix-apps-icons >/dev/null
    fi

    update-desktop-database \
        "$HOME/.local/share/applications" \
        >/dev/null 2>&1 || true

    systemctl --user restart \
        nodalix-shell.service \
        >/dev/null 2>&1 || true
}

# --- Nodalix overlay reconciliation ---

NODALIX_SELECTED_COMMITS=()
NODALIX_OVERLAY_HASH="none"
NODALIX_OVERLAY_ID="none"

nodalix_overlay_hash_commits() {
    if [ "$#" -eq 0 ]; then
        printf "none\n"
        return 0
    fi

    local payload=""
    local commit=""
    local patch_id=""

    for commit in "$@"; do
        patch_id="$(
            git show --pretty=format: --binary "$commit" |
            git patch-id --stable |
            awk "{print \$1}"
        )"

        if [ -z "$patch_id" ]; then
            patch_id="$commit"
        fi

        payload+="${patch_id}"$'\n'
    done

    printf "%s" "$payload" | sha256sum | awk "{print \$1}"
}

nodalix_commit_touched_files() {
    local commit="$1"

    git diff-tree \
        --root \
        --no-commit-id \
        --name-only \
        -r \
        -M \
        "$commit" \
        -- |
        sed "/^[[:space:]]*$/d" |
        sort -u
}

nodalix_upstream_changed_files() {
    local upstream_ref="$1"
    local overlay_ref="$2"
    local base=""

    base="$(git merge-base "$upstream_ref" "$overlay_ref")"

    if [ -z "$base" ]; then
        echo "ERROR: no se pudo encontrar merge-base entre upstream y overlay." >&2
        return 1
    fi

    git diff \
        --name-only \
        -M \
        "$base" \
        "$upstream_ref" \
        -- |
        sed "/^[[:space:]]*$/d" |
        sort -u
}

nodalix_overlay_commit_overlap() {
    local upstream_ref="$1"
    local overlay_ref="$2"
    local commit="$3"

    local upstream_tmp=""
    local local_tmp=""

    upstream_tmp="$(mktemp)"
    local_tmp="$(mktemp)"

    nodalix_upstream_changed_files \
        "$upstream_ref" \
        "$overlay_ref" \
        > "$upstream_tmp"

    nodalix_commit_touched_files \
        "$commit" \
        > "$local_tmp"

    comm -12 \
        "$upstream_tmp" \
        "$local_tmp"

    rm -f "$upstream_tmp" "$local_tmp"
}

nodalix_reconcile_choice() {
    local app="$1"
    local upstream_ref="$2"
    local commit="$3"
    shift 3

    local overlaps=("$@")
    local upstream_commit=""
    local state_root=""
    local decision_file=""
    local decision=""
    local policy="${NODALIX_OVERLAY_POLICY:-auto}"

    upstream_commit="$(git rev-parse "$upstream_ref")"

    state_root="$HOME/.local/state/nodalix-apps/reconciliation/$app/$upstream_commit"
    decision_file="$state_root/$commit"

    if [ -f "$decision_file" ]; then
        decision="$(cat "$decision_file")"

        case "$decision" in
            local|upstream)
                printf "%s\n" "$decision"
                return 0
                ;;
        esac
    fi

    if [ "$policy" = "upstream" ]; then
        printf "upstream\n"
        return 0
    fi

    if [ "$policy" = "local" ]; then
        printf "local\n"
        return 0
    fi

    # auto = preguntar si tenemos terminal; si no, upstream gana.
    if [ "$policy" = "auto" ] && [ ! -r /dev/tty ]; then
        printf "upstream\n"
        return 0
    fi

    if [ "$policy" = "auto" ] || [ "$policy" = "ask" ]; then
        local subject=""
        local answer=""

        subject="$(git show -s --format=%s "$commit")"

        {
            echo
            echo "⚠ Nodalix detectó un solapamiento con upstream"
            echo
            echo "Aplicación: $app"
            echo "Commit local:"
            echo "  ${commit:0:12}  $subject"
            echo
            echo "Upstream también modificó:"
            local file=""
            for file in "${overlaps[@]}"; do
                echo "  $file"
            done
            echo
            echo "Upstream tiene prioridad por defecto."
            printf "¿Quieres CONSERVAR este cambio local sobre upstream? [s/N] "
        } > /dev/tty

        IFS= read -r answer < /dev/tty || answer=""

        case "${answer,,}" in
            s|si|sí|y|yes)
                decision="local"
                ;;
            *)
                decision="upstream"
                ;;
        esac

        mkdir -p "$state_root"
        printf "%s\n" "$decision" > "$decision_file"

        printf "%s\n" "$decision"
        return 0
    fi

    echo "ERROR: política NODALIX_OVERLAY_POLICY desconocida: $policy" >&2
    return 1
}

nodalix_prepare_overlay() {
    local app="$1"
    local upstream_ref="$2"
    local overlay_ref="$3"

    local commit=""
    local choice=""
    local overlap_text=""
    local -a raw_commits=()
    local -a overlaps=()

    NODALIX_SELECTED_COMMITS=()
    NODALIX_OVERLAY_HASH="none"
    NODALIX_OVERLAY_ID="none"

    mapfile -t raw_commits < <(
        nodalix_overlay_commits \
            "$upstream_ref" \
            "$overlay_ref"
    )

    if [ "${#raw_commits[@]}" -eq 0 ]; then
        return 0
    fi

    for commit in "${raw_commits[@]}"; do
        overlaps=()

        overlap_text="$(
            nodalix_overlay_commit_overlap \
                "$upstream_ref" \
                "$overlay_ref" \
                "$commit"
        )"

        if [ -n "$overlap_text" ]; then
            mapfile -t overlaps <<< "$overlap_text"

            choice="$(
                nodalix_reconcile_choice \
                    "$app" \
                    "$upstream_ref" \
                    "$commit" \
                    "${overlaps[@]}"
            )"

            if [ "$choice" = "local" ]; then
                echo "==> Conservando cambio local ${commit:0:12}" >&2
                NODALIX_SELECTED_COMMITS+=("$commit")
            else
                echo "==> Usando upstream para ${commit:0:12}" >&2
            fi
        else
            NODALIX_SELECTED_COMMITS+=("$commit")
        fi
    done

    if [ "${#NODALIX_SELECTED_COMMITS[@]}" -gt 0 ]; then
        NODALIX_OVERLAY_HASH="$(
            nodalix_overlay_hash_commits \
                "${NODALIX_SELECTED_COMMITS[@]}"
        )"

        NODALIX_OVERLAY_ID="${NODALIX_OVERLAY_HASH:0:12}"
    fi
}

nodalix_show_prepared_overlay() {
    if [ "${#NODALIX_SELECTED_COMMITS[@]}" -eq 0 ]; then
        echo "  ninguno"
        return 0
    fi

    local commit=""

    for commit in "${NODALIX_SELECTED_COMMITS[@]}"; do
        git show \
            -s \
            --format="  %h  %s" \
            "$commit"
    done
}

nodalix_apply_prepared_overlay() {
    if [ "${#NODALIX_SELECTED_COMMITS[@]}" -eq 0 ]; then
        echo "==> No quedan cambios locales que aplicar."
        return 0
    fi

    local commit=""

    for commit in "${NODALIX_SELECTED_COMMITS[@]}"; do
        echo "==> Aplicando overlay ${commit:0:12}..."

        if ! git cherry-pick --empty=drop "$commit"; then
            git cherry-pick --abort >/dev/null 2>&1 || true

            echo >&2
            echo "ERROR: el cambio local no puede aplicarse limpiamente:" >&2
            git show -s --format="  %h  %s" "$commit" >&2
            echo >&2
            echo "La instalación activa NO se ha modificado." >&2
            return 1
        fi
    done
}

