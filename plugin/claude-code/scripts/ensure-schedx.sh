#!/usr/bin/env bash
set -uo pipefail

# ── Find schedx binary ───────────────────────────────────────────────────
SCHEDX_BIN=""
SCHEDX_VERSION=""

find_schedx() {
  if command -v schedx >/dev/null 2>&1; then
    SCHEDX_BIN="$(command -v schedx)"
  elif [ -x "$HOME/.local/bin/schedx" ]; then
    SCHEDX_BIN="$HOME/.local/bin/schedx"
  elif [ -x "$HOME/.cargo/bin/schedx" ]; then
    SCHEDX_BIN="$HOME/.cargo/bin/schedx"
  fi

  if [ -n "$SCHEDX_BIN" ]; then
    SCHEDX_VERSION="$("$SCHEDX_BIN" --version 2>/dev/null || echo "unknown")"
  fi
}

find_schedx

# ── Install if missing ────────────────────────────────────────────────────
if [ -z "$SCHEDX_BIN" ]; then
  echo "[schedx plugin] schedx not found, installing..." >&2

  INSTALL_SCRIPT="${CLAUDE_PLUGIN_ROOT}/scripts/install-schedx.sh"
  if [ -f "$INSTALL_SCRIPT" ]; then
    bash "$INSTALL_SCRIPT" >&2 2>&1 || true
  else
    curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh 2>/dev/null | sh >&2 2>&1 || true
  fi

  # Re-check after install
  find_schedx

  if [ -n "$SCHEDX_BIN" ]; then
    echo "[schedx plugin] installed $SCHEDX_VERSION at $SCHEDX_BIN" >&2
  else
    echo "[schedx plugin] auto-install failed. Install manually:" >&2
    echo "  curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh" >&2
  fi
fi

# ── Ensure install dir is in PATH for this session ────────────────────────
if [ -n "$SCHEDX_BIN" ] && [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  INSTALL_DIR="$(dirname "$SCHEDX_BIN")"
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "$CLAUDE_ENV_FILE" ;;
  esac
fi

# ── Output hook response ──────────────────────────────────────────────────
if [ -n "$SCHEDX_BIN" ]; then
  CONTEXT="schedx (${SCHEDX_VERSION}) is installed at ${SCHEDX_BIN}. Use the schedx skill for CLI reference. All schedx commands support --json for structured output."
else
  CONTEXT="schedx is NOT installed. To install: curl -fsSL https://raw.githubusercontent.com/Alireza29675/schedx/main/install.sh | sh"
fi

# Escape for JSON (handle quotes and backslashes)
CONTEXT_ESCAPED=$(printf '%s' "$CONTEXT" | sed 's/\\/\\\\/g; s/"/\\"/g')

cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "${CONTEXT_ESCAPED}"
  }
}
EOF

exit 0
